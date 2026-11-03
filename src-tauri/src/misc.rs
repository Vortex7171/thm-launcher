use crate::net::{self, HTTP};
use crate::store::{self, Settings, R, S};
use serde_json::{json, Value};

#[tauri::command]
pub fn get_settings() -> Settings { store::settings() }
#[tauri::command]
pub fn save_settings(settings: Settings) -> R<()> { store::save("settings.json", &settings) }
#[tauri::command]
pub fn list_activity() -> Vec<Value> { store::load("activity.json") }
#[tauri::command]
pub fn launcher_version() -> String { env!("CARGO_PKG_VERSION").into() }

/// Real status via mcsrvstat.us (public, key-free). Returns offline/unknown if unreachable.
#[tauri::command]
pub async fn server_status(address: String) -> Value {
    let a = address.trim();
    if a.is_empty() { return json!({ "online": false }); }
    match net::get_json(&format!("https://api.mcsrvstat.us/3/{a}")).await { Ok(v) => v, Err(_) => json!({ "online": false, "error": true }) }
}

/// Announcements come only from a user/team-configured JSON URL: [{ "title", "body", "date" }]
#[tauri::command]
pub async fn announcements() -> R<Value> {
    let u = store::settings().announcements_url;
    if u.trim().is_empty() { return Ok(json!([])); }
    net::get_json(u.trim()).await
}

fn newer(a: &str, b: &str) -> bool {
    let p = |s: &str| s.trim_start_matches('v').split('.').map(|x| x.parse::<u32>().unwrap_or(0)).collect::<Vec<_>>();
    p(a) > p(b)
}

/// Shows what would change; applies nothing.
#[tauri::command]
pub async fn check_updates(id: String) -> R<Value> {
    let inst = store::instance(&id)?;
    let mut out = json!({});
    let u = store::settings().launcher_update_url;
    if !u.trim().is_empty() {
        let m = net::get_json(u.trim()).await?; // { version, notes, url }
        let cur = env!("CARGO_PKG_VERSION");
        out["launcher"] = json!({ "current": cur, "latest": m["version"], "available": newer(m["version"].as_str().unwrap_or("0"), cur), "notes": m["notes"], "url": m["url"] });
    }
    let latest = crate::mc::release_versions().await?.into_iter().next();
    out["minecraft"] = json!({ "current": inst.mc_version, "latest_release": latest, "newer": latest.as_deref().map(|l| newer(l, &inst.mc_version)).unwrap_or(false),
        "note": "Changing Minecraft version can break mods — clone the instance first." });
    if inst.loader == "fabric" {
        let l = crate::mc::latest_fabric_loader(&inst.mc_version).await.ok();
        out["fabric"] = json!({ "current": inst.loader_version, "latest": l, "available": l.is_some() && l != inst.loader_version });
    }
    out["mods"] = json!(crate::mods::check_mod_updates(id).await.unwrap_or_default());
    Ok(out)
}
#[tauri::command]
pub fn apply_loader_update(id: String) -> R<()> {
    let mut v = store::instances();
    let i = v.iter_mut().find(|i| i.id == id).ok_or("not found")?;
    i.loader_version = None; i.installed = false; // next Play re-resolves the latest stable Fabric
    store::save_instances(&v)
}
#[allow(dead_code)]
fn _unused() { let _ = HTTP.clone(); let _: Result<(), String> = Err::<(), _>("x").s(); }
