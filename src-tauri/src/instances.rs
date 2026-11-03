use crate::store::{self, Instance, R, S};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn data_instances() -> PathBuf { store::dir().join("instances") }

pub fn new_instance(name: &str, mc: &str, loader: &str, kind: &str) -> Instance {
    let id = uuid::Uuid::new_v4().to_string();
    let gd = data_instances().join(&id).join("minecraft");
    Instance { id, name: name.into(), kind: kind.into(), mc_version: mc.into(), loader: loader.into(),
        game_dir: gd.display().to_string(), created_at: chrono::Utc::now().to_rfc3339(), ..Default::default() }
}
fn add(i: Instance) -> R<Instance> {
    fs::create_dir_all(&i.game_dir).s()?;
    let mut v = store::instances(); v.push(i.clone()); store::save_instances(&v)?; Ok(i)
}
pub fn ensure_default() -> R<()> {
    let m = crate::thm::manifest();
    let mut all = store::instances();
    if !all.is_empty() {
        let want = m["minecraft"].as_str().unwrap_or("1.21.11").to_string();
        for i in all.iter_mut().filter(|i| i.kind == "thm" && !i.installed) { i.mc_version = want.clone(); i.loader_version = None; }
        return store::save_instances(&all);
    }
    let mut i = new_instance("THM", m["minecraft"].as_str().unwrap_or("1.21.11"), "fabric", "thm");
    i.server = m["server"].as_str().unwrap_or("play.6b6t.org").into();
    i.memory_mb = 6144;
    add(i)?; store::activity("Created default THM instance"); Ok(())
}

#[tauri::command]
pub fn list_instances() -> Vec<Instance> { store::instances() }

#[tauri::command]
pub async fn list_mc_versions() -> R<Vec<String>> { crate::mc::release_versions().await }

#[tauri::command]
pub fn create_instance(name: String, mc_version: String, loader: String) -> R<Instance> {
    if name.trim().is_empty() { return Err("Name required".into()); }
    if loader != "fabric" && loader != "vanilla" { return Err("Unsupported loader".into()); }
    add(new_instance(name.trim(), &mc_version, &loader, "custom"))
}
#[tauri::command]
pub fn rename_instance(id: String, name: String) -> R<()> {
    let mut v = store::instances();
    v.iter_mut().find(|i| i.id == id).ok_or("not found")?.name = name.trim().into();
    store::save_instances(&v)
}
#[tauri::command]
pub fn update_instance(instance: Instance) -> R<()> {
    let mut v = store::instances();
    let cur = v.iter_mut().find(|i| i.id == instance.id).ok_or("not found")?;
    // Only user-editable fields; identity, paths and install state stay authoritative.
    cur.memory_mb = instance.memory_mb.clamp(512, 65536); cur.jvm_args = instance.jvm_args; cur.java_path = instance.java_path;
    cur.width = instance.width; cur.height = instance.height; cur.fullscreen = instance.fullscreen; cur.server = instance.server;
    store::save_instances(&v)
}
#[tauri::command]
pub fn delete_instance(id: String, delete_files: bool) -> R<()> {
    if crate::launch::is_running(&id) { return Err("Stop the instance first".into()); }
    let mut v = store::instances();
    let i = v.iter().find(|i| i.id == id).cloned().ok_or("not found")?;
    v.retain(|x| x.id != id); store::save_instances(&v)?;
    if delete_files {
        // Only ever delete inside the launcher's own instances folder.
        let root = PathBuf::from(&i.game_dir);
        let inst_root = root.parent().unwrap_or(&root).to_path_buf();
        if inst_root.starts_with(data_instances()) { fs::remove_dir_all(inst_root).ok(); }
    }
    Ok(())
}
fn copy_dir(src: &Path, dst: &Path) -> R<()> {
    for e in WalkDir::new(src).follow_links(false) {
        let e = e.s()?; let rel = e.path().strip_prefix(src).s()?; let t = dst.join(rel);
        if e.file_type().is_dir() { fs::create_dir_all(&t).s()?; } else if e.file_type().is_file() { fs::copy(e.path(), &t).s()?; }
    }
    Ok(())
}
#[tauri::command]
pub fn clone_instance(id: String, new_name: String) -> R<Instance> {
    let src = store::instance(&id)?;
    let mut n = new_instance(new_name.trim(), &src.mc_version, &src.loader, "custom");
    n.loader_version = src.loader_version; n.memory_mb = src.memory_mb; n.jvm_args = src.jvm_args; n.java_path = src.java_path;
    n.width = src.width; n.height = src.height; n.fullscreen = src.fullscreen; n.server = src.server; n.installed = src.installed;
    fs::create_dir_all(&n.game_dir).s()?;
    copy_dir(Path::new(&src.game_dir), Path::new(&n.game_dir))?;
    let mut v = store::instances(); v.push(n.clone()); store::save_instances(&v)?; Ok(n)
}
#[tauri::command]
pub fn repair_instance(id: String) -> R<()> {
    // Forces re-verification: install() re-hashes every file and re-downloads anything missing or corrupt.
    let mut v = store::instances();
    let i = v.iter_mut().find(|i| i.id == id).ok_or("not found")?;
    i.installed = false; store::save_instances(&v)
}
#[tauri::command]
pub fn export_instance(id: String, dest: String) -> R<()> {
    use std::io::Write;
    let i = store::instance(&id)?;
    let f = fs::File::create(&dest).s()?;
    let mut z = zip::ZipWriter::new(f);
    let o = zip::write::SimpleFileOptions::default();
    z.start_file("instance.json", o).s()?; z.write_all(&serde_json::to_vec_pretty(&i).s()?).s()?;
    let root = Path::new(&i.game_dir);
    for e in WalkDir::new(root).follow_links(false).into_iter().filter_map(|e| e.ok()).filter(|e| e.file_type().is_file()) {
        let rel = e.path().strip_prefix(root).s()?.to_string_lossy().replace('\\', "/");
        if rel.starts_with("logs/") || rel.starts_with("crash-reports/") { continue; }
        z.start_file(format!("minecraft/{rel}"), o).s()?; z.write_all(&fs::read(e.path()).s()?).s()?;
    }
    z.finish().s()?; Ok(())
}
#[tauri::command]
pub fn import_instance(src: String) -> R<Instance> {
    let mut z = zip::ZipArchive::new(fs::File::open(&src).s()?).s()?;
    let meta: Instance = { let mut f = z.by_name("instance.json").map_err(|_| "Not a THM instance export".to_string())?; serde_json::from_reader(&mut f).s()? };
    let mut n = new_instance(&format!("{} (imported)", meta.name), &meta.mc_version, &meta.loader, "custom");
    n.loader_version = meta.loader_version; n.memory_mb = meta.memory_mb; n.jvm_args = meta.jvm_args; n.server = meta.server;
    let gd = PathBuf::from(&n.game_dir); fs::create_dir_all(&gd).s()?;
    for k in 0..z.len() {
        let mut f = z.by_index(k).s()?;
        let Some(rel) = f.enclosed_name() else { continue }; // rejects ../ traversal
        let Ok(rel) = rel.strip_prefix("minecraft") else { continue };
        if f.is_dir() { continue; }
        let out = gd.join(rel);
        if let Some(p) = out.parent() { fs::create_dir_all(p).s()?; }
        std::io::copy(&mut f, &mut fs::File::create(out).s()?).s()?;
    }
    let mut v = store::instances(); v.push(n.clone()); store::save_instances(&v)?; Ok(n)
}
