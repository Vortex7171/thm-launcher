use crate::net::{self, HTTP};
use crate::store::{self, safe_name, Instance, R, S};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;

pub fn mods_dir(i: &Instance) -> PathBuf { let d = PathBuf::from(&i.game_dir).join("mods"); std::fs::create_dir_all(&d).ok(); d }

#[derive(Serialize, Clone)]
pub struct ModInfo { file: String, enabled: bool, size: u64, id: Option<String>, name: Option<String>, version: Option<String>,
    description: Option<String>, sha1: String, depends: Vec<String>, provides: Vec<String>, mc_req: Option<String> }

fn read_meta(p: &std::path::Path) -> Option<Value> {
    let mut z = zip::ZipArchive::new(std::fs::File::open(p).ok()?).ok()?;
    let mut s = String::new(); z.by_name("fabric.mod.json").ok()?.read_to_string(&mut s).ok()?;
    serde_json::from_str(&s).ok()
}
fn ver(s: &str) -> Vec<u32> { s.split('.').map(|p| p.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0)).collect() }
fn one(req: &str, mc: &str) -> bool {
    let m = ver(mc);
    req.split_whitespace().all(|p| {
        if p == "*" { true }
        else if let Some(x) = p.strip_prefix(">=") { m >= ver(x) }
        else if let Some(x) = p.strip_prefix('>') { m > ver(x) }
        else if let Some(x) = p.strip_prefix("<=") { m <= ver(x) || m.starts_with(&ver(x)) }
        else if let Some(x) = p.strip_prefix('<') { m < ver(x) }
        else if let Some(x) = p.strip_prefix('~').or(p.strip_prefix('^')) { let v = ver(x); m.len() >= 2 && v.len() >= 2 && m[0] == v[0] && m[1] == v[1] }
        else if p.contains('x') || p.contains('*') { let pre: Vec<&str> = p.split('.').take_while(|s| *s != "x" && *s != "*").collect(); mc.split('.').zip(pre.iter()).all(|(a, b)| a == *b) }
        else { p.trim_start_matches('=') == mc }
    })
}
fn mc_ok(req: &str, mc: &str) -> bool { req.split("||").any(|r| one(r.trim(), mc)) }

fn scan(i: &Instance) -> R<Vec<ModInfo>> {
    let mut out = vec![];
    for e in std::fs::read_dir(mods_dir(i)).s()?.filter_map(|e| e.ok()) {
        let p = e.path(); let file = e.file_name().to_string_lossy().to_string();
        let enabled = file.ends_with(".jar");
        if !enabled && !file.ends_with(".jar.disabled") { continue; }
        let m = read_meta(&p);
        let g = |k: &str| m.as_ref().and_then(|m| m[k].as_str()).map(String::from);
        let depends: Vec<String> = m.as_ref().and_then(|m| m["depends"].as_object()).map(|o| o.keys().cloned().collect()).unwrap_or_default();
        let mc_req = m.as_ref().and_then(|m| m.pointer("/depends/minecraft")).map(|v| match v { Value::String(s) => s.clone(), Value::Array(a) => a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(" || "), _ => String::new() });
        let provides = m.as_ref().and_then(|m| m["provides"].as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
        out.push(ModInfo { size: e.metadata().map(|m| m.len()).unwrap_or(0), sha1: net::sha1_hex(&std::fs::read(&p).s()?),
            id: g("id"), name: g("name"), version: g("version"), description: g("description"), file, enabled, depends, provides, mc_req });
    }
    out.sort_by(|a, b| a.file.to_lowercase().cmp(&b.file.to_lowercase())); Ok(out)
}

#[tauri::command]
pub fn analyze_mods(id: String) -> R<Value> {
    let inst = store::instance(&id)?; let mods = scan(&inst)?;
    let on: Vec<&ModInfo> = mods.iter().filter(|m| m.enabled).collect();
    let mut have: HashMap<String, usize> = HashMap::new();
    for m in &on { if let Some(i) = &m.id { *have.entry(i.clone()).or_default() += 1; } for p in &m.provides { *have.entry(p.clone()).or_default() += 1; } }
    let builtin = ["minecraft", "java", "fabricloader", "fabric-loader"];
    let fapi = have.contains_key("fabric-api");
    let dupes: Vec<Value> = on.iter().filter_map(|m| m.id.as_ref()).filter(|i| have[*i] > 1).collect::<std::collections::BTreeSet<_>>().into_iter()
        .map(|i| json!({ "id": i, "files": on.iter().filter(|m| m.id.as_ref() == Some(i)).map(|m| m.file.clone()).collect::<Vec<_>>() })).collect();
    let mut missing = vec![]; let mut incompat = vec![];
    for m in &on {
        for d in &m.depends { if !builtin.contains(&d.as_str()) && !have.contains_key(d) && !(fapi && d.starts_with("fabric-")) { missing.push(json!({ "mod": m.file, "dep": d })); } }
        if let Some(r) = &m.mc_req { if !r.is_empty() && !mc_ok(r, &inst.mc_version) { incompat.push(json!({ "mod": m.file, "requires": r, "minecraft": inst.mc_version })); } }
    }
    Ok(json!({ "mods": mods, "duplicates": dupes, "missing": missing, "incompatible": incompat }))
}

#[tauri::command]
pub fn add_mod_jar(id: String, src: String) -> R<String> {
    let inst = store::instance(&id)?; let s = PathBuf::from(&src);
    if s.extension().and_then(|e| e.to_str()) != Some("jar") { return Err("Only .jar files can be imported".into()); }
    if read_meta(&s).is_none() { return Err("This JAR has no fabric.mod.json — it is not a Fabric mod for this loader.".into()); }
    let name = s.file_name().ok_or("bad file")?.to_string_lossy().to_string();
    std::fs::copy(&s, mods_dir(&inst).join(&name)).s()?;
    store::activity(&format!("Imported {name} into {}", inst.name)); Ok(name)
}
#[tauri::command]
pub fn toggle_mod(id: String, file: String, enabled: bool) -> R<()> {
    let d = mods_dir(&store::instance(&id)?); let f = safe_name(&file)?;
    let dest = if enabled { f.trim_end_matches(".disabled").to_string() } else if f.ends_with(".jar") { format!("{f}.disabled") } else { f.to_string() };
    std::fs::rename(d.join(f), d.join(dest)).s()
}
#[tauri::command]
pub fn delete_mod(id: String, file: String) -> R<()> {
    let d = mods_dir(&store::instance(&id)?); std::fs::remove_file(d.join(safe_name(&file)?)).s()
}
#[tauri::command]
pub fn export_mods(id: String, dest: String) -> R<()> {
    use std::io::Write;
    let d = mods_dir(&store::instance(&id)?);
    let mut z = zip::ZipWriter::new(std::fs::File::create(dest).s()?);
    for m in std::fs::read_dir(d).s()?.filter_map(|e| e.ok()) {
        let n = m.file_name().to_string_lossy().to_string();
        if n.ends_with(".jar") || n.ends_with(".jar.disabled") { z.start_file(n, zip::write::SimpleFileOptions::default()).s()?; z.write_all(&std::fs::read(m.path()).s()?).s()?; }
    }
    z.finish().s()?; Ok(())
}

// ---------- Modrinth ----------
#[tauri::command]
pub async fn modrinth_search(id: String, query: String) -> R<Value> {
    let inst = store::instance(&id)?;
    let facets = format!("[[\"project_type:mod\"],[\"categories:fabric\"],[\"versions:{}\"]]", inst.mc_version);
    let r = HTTP.get("https://api.modrinth.com/v2/search").query(&[("query", query.as_str()), ("facets", &facets), ("limit", "20")]).send().await.s()?.error_for_status().s()?.json::<Value>().await.s()?;
    Ok(r["hits"].clone())
}

pub async fn install_project(inst: &Instance, project: &str, depth: u8) -> R<Vec<String>> {
    let vs: Value = HTTP.get(format!("https://api.modrinth.com/v2/project/{project}/version")).query(&[("loaders", "[\"fabric\"]".to_string()), ("game_versions", format!("[\"{}\"]", inst.mc_version))])
        .send().await.s()?.error_for_status().s()?.json().await.s()?;
    let v = vs.get(0).ok_or("No Fabric build of this project for this Minecraft version")?;
    let f = v["files"].as_array().and_then(|a| a.iter().find(|f| f["primary"] == true).or(a.first())).ok_or("no files")?;
    let name = safe_name(f["filename"].as_str().ok_or("bad file")?)?.to_string();
    net::download(f["url"].as_str().ok_or("bad url")?, &mods_dir(inst).join(&name), f["hashes"]["sha1"].as_str()).await?;
    let mut got = vec![name];
    if depth < 2 {
        for d in v["dependencies"].as_array().cloned().unwrap_or_default() {
            if d["dependency_type"] == "required" { if let Some(p) = d["project_id"].as_str() {
                let present = scan(inst)?.iter().any(|m| m.id.as_deref() == Some(p));
                if !present { if let Ok(mut x) = Box::pin(install_project(inst, p, depth + 1)).await { got.append(&mut x); } }
            } }
        }
    }
    Ok(got)
}
#[tauri::command]
pub async fn modrinth_install(id: String, project: String) -> R<Vec<String>> {
    let inst = store::instance(&id)?; let r = install_project(&inst, &project, 0).await?;
    store::activity(&format!("Installed {} from Modrinth", r.join(", "))); Ok(r)
}

/// Update check by file hash (Modrinth's version_files/update).
#[tauri::command]
pub async fn check_mod_updates(id: String) -> R<Vec<Value>> {
    let inst = store::instance(&id)?; let mods: Vec<ModInfo> = scan(&inst)?.into_iter().filter(|m| m.enabled).collect();
    if mods.is_empty() { return Ok(vec![]); }
    let body = json!({ "hashes": mods.iter().map(|m| m.sha1.clone()).collect::<Vec<_>>(), "algorithm": "sha1", "loaders": ["fabric"], "game_versions": [inst.mc_version] });
    let r: Value = HTTP.post("https://api.modrinth.com/v2/version_files/update").json(&body).send().await.s()?.error_for_status().s()?.json().await.s()?;
    let mut out = vec![];
    for m in mods {
        if let Some(v) = r.get(&m.sha1) {
            let f = v["files"].as_array().and_then(|a| a.iter().find(|f| f["primary"] == true).or(a.first()));
            if let Some(f) = f { if f["hashes"]["sha1"].as_str() != Some(&m.sha1) {
                out.push(json!({ "file": m.file, "current": m.version, "new_version": v["version_number"], "url": f["url"], "sha1": f["hashes"]["sha1"], "filename": f["filename"] }));
            } }
        }
    }
    Ok(out)
}
/// Replaces a mod with its newer build; the old JAR is kept as .bak so nothing is lost.
#[tauri::command]
pub async fn apply_mod_update(id: String, file: String, url: String, sha1: String, filename: String) -> R<()> {
    let d = mods_dir(&store::instance(&id)?); let (old, new) = (safe_name(&file)?, safe_name(&filename)?);
    net::download(&url, &d.join(format!("{new}.new")), Some(&sha1)).await?;
    let bak = d.join(format!("{old}.bak")); std::fs::rename(d.join(old), &bak).s()?;
    std::fs::rename(d.join(format!("{new}.new")), d.join(new)).s()?;
    store::activity(&format!("Updated {old} → {new} (old build saved as .bak)")); Ok(())
}
