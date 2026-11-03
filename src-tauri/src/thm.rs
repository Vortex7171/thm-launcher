use crate::net::{self, HTTP};
use crate::store::{self, safe_name, R, S};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const BUNDLED: &str = include_str!("../thm-manifest.json");

/// Bundled manifest, overridden by a cached copy of the user-configured manifest URL if present.
pub fn manifest() -> Value {
    let cached: Value = store::load("thm-manifest-cache.json");
    if cached.get("slots").is_some() { cached } else { serde_json::from_str(BUNDLED).unwrap_or(json!({})) }
}

#[tauri::command]
pub fn thm_manifest() -> Value { manifest() }

#[tauri::command]
pub async fn refresh_manifest() -> R<Value> {
    let url = store::settings().thm_manifest_url;
    if url.trim().is_empty() { return Err("No THM manifest URL configured in Settings".into()); }
    let m = net::get_json(url.trim()).await?;
    if m.get("slots").is_none() { return Err("That URL did not return a valid THM manifest".into()); }
    store::save("thm-manifest-cache.json", &m)?; store::activity("Refreshed THM manifest"); Ok(m)
}

#[tauri::command]
pub fn thm_slots(id: String) -> R<Value> {
    let inst = store::instance(&id)?; let s = store::settings();
    let files: Vec<String> = std::fs::read_dir(crate::mods::mods_dir(&inst)).s()?.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_lowercase()).filter(|f| f.ends_with(".jar")).collect();
    let slots: Vec<Value> = manifest()["slots"].as_array().cloned().unwrap_or_default().into_iter().map(|mut sl| {
        let m = sl["match"].as_str().unwrap_or("\u{0}").to_lowercase();
        sl["installed"] = json!(files.iter().any(|f| f.contains(&m)));
        let url = s.slot_urls.get(sl["id"].as_str().unwrap_or("")).cloned().filter(|u| !u.is_empty()).or_else(|| sl["url"].as_str().map(String::from));
        sl["can_download"] = json!(sl["source"] == "modrinth" || url.is_some());
        sl
    }).collect();
    Ok(json!(slots))
}

pub async fn install_slot(inst: &store::Instance, sl: &Value) -> R<String> {
    let slot = sl["id"].as_str().unwrap_or("").to_string();
    if sl["source"] == "modrinth" {
        return Ok(crate::mods::install_project(inst, sl["project"].as_str().unwrap_or(""), 0).await?.join(", "));
    }
    let url = store::settings().slot_urls.get(&slot).cloned().filter(|u| !u.is_empty()).or_else(|| sl["url"].as_str().map(String::from))
        .ok_or("No download source is configured for this component. Import its JAR, or set a source URL in Settings → THM sources.")?;
    let bytes = HTTP.get(&url).send().await.s()?.error_for_status().s()?.bytes().await.s()?;
    if !bytes.starts_with(b"PK") { return Err("download was not a JAR file".into()); }
    if let Some(h) = sl["sha256"].as_str() { if !hex::encode(Sha256::digest(&bytes)).eq_ignore_ascii_case(h) { return Err("Checksum mismatch — download rejected".into()); } }
    let name = url.split('?').next().and_then(|u| u.rsplit('/').next()).filter(|n| n.ends_with(".jar")).map(String::from).unwrap_or(format!("{slot}.jar"));
    std::fs::write(crate::mods::mods_dir(inst).join(safe_name(&name)?), &bytes).s()?;
    store::activity(&format!("Installed {name}")); Ok(name)
}

#[tauri::command]
pub async fn thm_install_slot(id: String, slot: String) -> R<String> {
    let inst = store::instance(&id)?;
    let sl = manifest()["slots"].as_array().and_then(|a| a.iter().find(|s| s["id"] == slot.as_str()).cloned()).ok_or("unknown slot")?;
    install_slot(&inst, &sl).await
}

/// First-install step for the THM instance: downloads every component that isn't already present.
pub async fn install_missing(app: &tauri::AppHandle, inst: &store::Instance) -> R<()> {
    let slots = manifest()["slots"].as_array().cloned().unwrap_or_default();
    let files: Vec<String> = std::fs::read_dir(crate::mods::mods_dir(inst)).s()?.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_lowercase()).collect();
    let (total, mut errs) = (slots.len(), vec![]);
    for (n, sl) in slots.iter().enumerate() {
        net::progress(app, &inst.id, &format!("THM mods: {}", sl["name"].as_str().unwrap_or("")), n, total);
        let m = sl["match"].as_str().unwrap_or("\u{0}").to_lowercase();
        if files.iter().any(|f| f.contains(&m) && f.ends_with(".jar")) { continue; }
        if let Err(e) = install_slot(inst, sl).await { errs.push(format!("{}: {}", sl["name"].as_str().unwrap_or("?"), e)); }
    }
    net::progress(app, &inst.id, "THM mods", total, total);
    if errs.is_empty() { Ok(()) } else { Err(format!("Some THM mods failed to download (press Play to retry): {}", errs.join("; "))) }
}
