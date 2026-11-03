use crate::net::{self, progress, Job};
use crate::store::{self, Instance, R, S};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tauri::AppHandle;

const MANIFEST: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

pub fn os_name() -> &'static str { if cfg!(windows) { "windows" } else if cfg!(target_os = "macos") { "osx" } else { "linux" } }
pub fn arch() -> &'static str { if cfg!(target_arch = "aarch64") { "arm64" } else { "x86_64" } }

pub fn rules_ok(rules: Option<&Value>, feats: &[&str]) -> bool {
    let Some(rs) = rules.and_then(|r| r.as_array()) else { return true };
    let mut allowed = false;
    for r in rs {
        let mut m = true;
        if let Some(os) = r.get("os") {
            if let Some(n) = os["name"].as_str() { if n != os_name() { m = false; } }
            if let Some(a) = os["arch"].as_str() { if a != arch() { m = false; } }
        }
        if let Some(f) = r.get("features").and_then(|f| f.as_object()) {
            for (k, v) in f { if feats.contains(&k.as_str()) != v.as_bool().unwrap_or(false) { m = false; } }
        }
        if m { allowed = r["action"] == "allow"; }
    }
    allowed
}

fn maven_path(name: &str) -> String {
    let p: Vec<&str> = name.split(':').collect();
    if p.len() < 3 { return name.to_string(); }
    let cls = if p.len() > 3 { format!("-{}", p[3]) } else { String::new() };
    format!("{}/{}/{}/{}-{}{}.jar", p[0].replace('.', "/"), p[1], p[2], p[1], p[2], cls)
}
fn lib_key(name: &str) -> String { name.split(':').enumerate().filter(|(i, _)| *i != 2).map(|(_, s)| s).collect::<Vec<_>>().join(":") }

pub fn version_id(i: &Instance) -> String {
    match (&i.loader[..], &i.loader_version) { ("fabric", Some(lv)) => format!("{}-fabric-{}", i.mc_version, lv), _ => i.mc_version.clone() }
}
pub fn version_json_path(vid: &str) -> PathBuf { store::dir().join("versions").join(vid).join(format!("{vid}.json")) }

pub async fn release_versions() -> R<Vec<String>> {
    let m = net::get_json(MANIFEST).await?;
    Ok(m["versions"].as_array().map(|a| a.iter().filter(|v| v["type"] == "release").filter_map(|v| v["id"].as_str().map(String::from)).collect()).unwrap_or_default())
}
pub async fn latest_fabric_loader(mc: &str) -> R<String> {
    let l = net::get_json(&format!("https://meta.fabricmc.net/v2/versions/loader/{mc}")).await?;
    l.as_array().and_then(|a| a.iter().find(|e| e["loader"]["stable"] == true).or(a.first()))
        .and_then(|e| e["loader"]["version"].as_str()).map(String::from)
        .ok_or_else(|| "Fabric does not support this Minecraft version".to_string())
}

fn merge(mut base: Value, f: &Value) -> Value {
    let mut libs = f["libraries"].as_array().cloned().unwrap_or_default();
    libs.extend(base["libraries"].as_array().cloned().unwrap_or_default());
    base["libraries"] = Value::Array(libs);
    base["mainClass"] = f["mainClass"].clone();
    if base.get("arguments").is_some() {
        for k in ["jvm", "game"] {
            let mut a = base.pointer(&format!("/arguments/{k}")).and_then(|v| v.as_array()).cloned().unwrap_or_default();
            if let Some(x) = f.pointer(&format!("/arguments/{k}")).and_then(|v| v.as_array()) { a.extend(x.iter().cloned()); }
            base["arguments"][k] = Value::Array(a);
        }
    }
    base
}

pub struct Plan { pub jobs: Vec<Job>, pub classpath: Vec<PathBuf>, pub natives: Vec<PathBuf> }

pub fn plan_libraries(json: &Value) -> Plan {
    let libdir = store::dir().join("libraries");
    let (mut jobs, mut cp, mut natives, mut seen) = (vec![], vec![], vec![], HashSet::new());
    for lib in json["libraries"].as_array().cloned().unwrap_or_default() {
        if !rules_ok(lib.get("rules"), &[]) { continue; }
        let name = lib["name"].as_str().unwrap_or("").to_string();
        if let Some(n) = lib.pointer(&format!("/natives/{}", os_name())).and_then(|v| v.as_str()) {
            let key = n.replace("${arch}", if cfg!(target_pointer_width = "64") { "64" } else { "32" });
            if let Some(c) = lib.pointer(&format!("/downloads/classifiers/{key}")) {
                let dest = libdir.join(c["path"].as_str().unwrap_or("native.jar"));
                jobs.push(Job { url: c["url"].as_str().unwrap_or("").into(), dest: dest.clone(), sha1: c["sha1"].as_str().map(String::from) });
                natives.push(dest);
            }
        }
        if !seen.insert(lib_key(&name)) { continue; }
        if let Some(a) = lib.pointer("/downloads/artifact") {
            let dest = libdir.join(a["path"].as_str().unwrap_or(""));
            jobs.push(Job { url: a["url"].as_str().unwrap_or("").into(), dest: dest.clone(), sha1: a["sha1"].as_str().map(String::from) });
            cp.push(dest);
        } else if lib.get("downloads").is_none() {
            let mut base = lib["url"].as_str().unwrap_or("https://libraries.minecraft.net/").to_string();
            if !base.ends_with('/') { base.push('/'); }
            let path = maven_path(&name);
            jobs.push(Job { url: format!("{base}{path}"), dest: libdir.join(&path), sha1: None });
            cp.push(libdir.join(path));
        }
    }
    Plan { jobs, classpath: cp, natives }
}

fn extract_natives(files: &[PathBuf], out: &PathBuf) -> R<()> {
    std::fs::create_dir_all(out).s()?;
    for f in files {
        let mut z = zip::ZipArchive::new(std::fs::File::open(f).s()?).s()?;
        for i in 0..z.len() {
            let mut e = z.by_index(i).s()?;
            let Some(rel) = e.enclosed_name() else { continue };
            if e.is_dir() || rel.starts_with("META-INF") { continue; }
            let Some(fname) = rel.file_name() else { continue };
            std::io::copy(&mut e, &mut std::fs::File::create(out.join(fname)).s()?).s()?;
        }
    }
    Ok(())
}

/// Installs everything needed to launch. Returns (version id, loader version).
pub async fn install(app: &AppHandle, inst: &Instance) -> R<(String, Option<String>)> {
    let root = store::dir();
    let mc = &inst.mc_version;
    let id = &inst.id;
    progress(app, id, "Resolving versions", 0, 1);
    let vm = net::get_json(MANIFEST).await?;
    let entry = vm["versions"].as_array().and_then(|a| a.iter().find(|v| v["id"] == mc.as_str())).cloned().ok_or("Unknown Minecraft version")?;
    let vpath = version_json_path(mc);
    net::download(entry["url"].as_str().ok_or("bad manifest")?, &vpath, entry["sha1"].as_str()).await?;
    let vanilla: Value = serde_json::from_slice(&std::fs::read(&vpath).s()?).s()?;

    let (vid, lv, json) = if inst.loader == "fabric" {
        let lv = match &inst.loader_version { Some(v) => v.clone(), None => latest_fabric_loader(mc).await? };
        let profile = net::get_json(&format!("https://meta.fabricmc.net/v2/versions/loader/{mc}/{lv}/profile/json")).await?;
        let vid = format!("{mc}-fabric-{lv}");
        let merged = merge(vanilla, &profile);
        let p = version_json_path(&vid);
        std::fs::create_dir_all(p.parent().unwrap()).s()?;
        std::fs::write(&p, serde_json::to_vec(&merged).s()?).s()?;
        (vid, Some(lv), merged)
    } else { (mc.clone(), None, vanilla) };

    let major = json["javaVersion"]["majorVersion"].as_u64().unwrap_or(8);
    crate::java::ensure(app, inst, major).await?;

    let mut jobs = vec![];
    let client = json.pointer("/downloads/client").ok_or("version has no client jar")?;
    let client_jar = root.join("versions").join(mc).join(format!("{mc}.jar"));
    jobs.push(Job { url: client["url"].as_str().unwrap_or("").into(), dest: client_jar, sha1: client["sha1"].as_str().map(String::from) });
    let plan = plan_libraries(&json);
    jobs.extend(plan.jobs.clone());
    net::download_all(app, id, "Minecraft & libraries", jobs).await?;
    extract_natives(&plan.natives, &root.join("versions").join(&vid).join("natives"))?;

    let ai = &json["assetIndex"];
    let aid = ai["id"].as_str().ok_or("no asset index")?;
    let idx_path = root.join("assets").join("indexes").join(format!("{aid}.json"));
    net::download(ai["url"].as_str().unwrap_or(""), &idx_path, ai["sha1"].as_str()).await?;
    let idx: Value = serde_json::from_slice(&std::fs::read(&idx_path).s()?).s()?;
    let mut ajobs = vec![];
    if let Some(o) = idx["objects"].as_object() {
        for (_, v) in o {
            let h = v["hash"].as_str().unwrap_or("");
            if h.len() < 2 { continue; }
            ajobs.push(Job { url: format!("https://resources.download.minecraft.net/{}/{}", &h[..2], h), dest: root.join("assets").join("objects").join(&h[..2]).join(h), sha1: Some(h.to_string()) });
        }
    }
    net::download_all(app, id, "Game assets", ajobs).await?;
    for d in ["mods", "config", "saves", "resourcepacks", "shaderpacks", "logs"] { std::fs::create_dir_all(PathBuf::from(&inst.game_dir).join(d)).s()?; }
    progress(app, id, "Install complete", 1, 1);
    Ok((vid, lv))
}

pub struct Session { pub name: String, pub uuid: String, pub token: String, pub user_type: String }
pub struct Cmd { pub args: Vec<String>, pub dir: PathBuf }

fn sub(s: &str, vars: &HashMap<&str, String>) -> String {
    let mut o = s.to_string();
    for (k, v) in vars { o = o.replace(&format!("${{{k}}}"), v); }
    o
}
fn expand(v: Option<&Value>, feats: &[&str], vars: &HashMap<&str, String>) -> Vec<String> {
    let mut out = vec![];
    for it in v.and_then(|v| v.as_array()).cloned().unwrap_or_default() {
        match &it {
            Value::String(s) => out.push(sub(s, vars)),
            Value::Object(_) if rules_ok(it.get("rules"), feats) => match &it["value"] {
                Value::String(s) => out.push(sub(s, vars)),
                Value::Array(a) => for x in a { if let Some(s) = x.as_str() { out.push(sub(s, vars)); } },
                _ => {}
            },
            _ => {}
        }
    }
    out
}

pub fn build_command(inst: &Instance, sess: &Session, java_major_out: &mut u64) -> R<Cmd> {
    let root = store::dir();
    let vid = version_id(inst);
    let json: Value = serde_json::from_slice(&std::fs::read(version_json_path(&vid)).map_err(|_| "Not installed yet — install the instance first".to_string())?).s()?;
    *java_major_out = json["javaVersion"]["majorVersion"].as_u64().unwrap_or(8);
    let plan = plan_libraries(&json);
    let sep = if cfg!(windows) { ";" } else { ":" };
    let mut cp: Vec<String> = plan.classpath.iter().map(|p| p.display().to_string()).collect();
    cp.push(root.join("versions").join(&inst.mc_version).join(format!("{}.jar", inst.mc_version)).display().to_string());

    let server = if inst.server.trim().is_empty() { store::settings().default_server } else { inst.server.clone() };
    let quick = json.to_string().contains("quickPlayMultiplayer");
    let mut feats: Vec<&str> = vec![];
    if !inst.fullscreen { feats.push("has_custom_resolution"); }
    if quick && !server.is_empty() { feats.push("is_quick_play_multiplayer"); }

    let mut v: HashMap<&str, String> = HashMap::new();
    v.insert("auth_player_name", sess.name.clone()); v.insert("auth_uuid", sess.uuid.clone());
    v.insert("auth_access_token", sess.token.clone()); v.insert("auth_session", sess.token.clone());
    v.insert("user_type", sess.user_type.clone()); v.insert("clientid", String::new()); v.insert("auth_xuid", String::new());
    v.insert("version_name", vid.clone()); v.insert("version_type", "THM-Launcher".into());
    v.insert("game_directory", inst.game_dir.clone());
    v.insert("assets_root", root.join("assets").display().to_string());
    v.insert("assets_index_name", json["assetIndex"]["id"].as_str().unwrap_or("").into());
    v.insert("user_properties", "{}".into());
    v.insert("resolution_width", inst.width.to_string()); v.insert("resolution_height", inst.height.to_string());
    v.insert("natives_directory", root.join("versions").join(&vid).join("natives").display().to_string());
    v.insert("launcher_name", "THM-Launcher".into()); v.insert("launcher_version", env!("CARGO_PKG_VERSION").into());
    v.insert("classpath", cp.join(sep)); v.insert("classpath_separator", sep.into());
    v.insert("library_directory", root.join("libraries").display().to_string());
    v.insert("quickPlayMultiplayer", server.clone());

    let mut args = vec![format!("-Xmx{}M", inst.memory_mb), format!("-Xms{}M", (inst.memory_mb / 4).max(512))];
    args.extend(inst.jvm_args.split_whitespace().map(String::from));
    if json.pointer("/arguments/jvm").is_some() {
        args.extend(expand(json.pointer("/arguments/jvm"), &feats, &v));
    } else {
        args.push(format!("-Djava.library.path={}", v["natives_directory"])); args.push("-cp".into()); args.push(cp.join(sep));
    }
    args.push(json["mainClass"].as_str().ok_or("no main class")?.into());
    if json.pointer("/arguments/game").is_some() {
        args.extend(expand(json.pointer("/arguments/game"), &feats, &v));
    } else if let Some(m) = json["minecraftArguments"].as_str() {
        args.extend(m.split_whitespace().map(|s| sub(s, &v)));
    }
    if !quick && !server.is_empty() {
        let (h, p) = server.split_once(':').unwrap_or((&server, "25565"));
        args.extend(["--server".into(), h.into(), "--port".into(), p.into()]);
    }
    if inst.fullscreen { args.push("--fullscreen".into()); }
    Ok(Cmd { args, dir: PathBuf::from(&inst.game_dir) })
}
