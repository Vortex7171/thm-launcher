use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

pub type R<T> = Result<T, String>;
pub trait S<T> { fn s(self) -> R<T>; }
impl<T, E: std::fmt::Display> S<T> for Result<T, E> {
    fn s(self) -> R<T> { self.map_err(|e| e.to_string()) }
}

pub static DIR: OnceCell<PathBuf> = OnceCell::new();
static LOCK: Mutex<()> = Mutex::new(());
pub fn dir() -> PathBuf { DIR.get().expect("data dir").clone() }

pub fn load<T: for<'a> Deserialize<'a> + Default>(name: &str) -> T {
    std::fs::read_to_string(dir().join(name)).ok()
        .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}
/// Atomic write (tmp + rename) so a crash never corrupts config.
pub fn save<T: Serialize>(name: &str, v: &T) -> R<()> {
    let _g = LOCK.lock().unwrap();
    let p = dir().join(name);
    let tmp = dir().join(format!("{name}.tmp"));
    std::fs::write(&tmp, serde_json::to_vec_pretty(v).s()?).s()?;
    std::fs::rename(&tmp, &p).s()
}
/// Reject names that could escape a directory.
pub fn safe_name(n: &str) -> R<&str> {
    if n.is_empty() || n.contains('/') || n.contains('\\') || n.contains("..") { Err("invalid file name".into()) } else { Ok(n) }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub ms_client_id: String,
    pub launcher_update_url: String,
    pub thm_manifest_url: String,
    pub announcements_url: String,
    pub java_path: String,
    pub default_server: String,
    pub slot_urls: HashMap<String, String>,
    pub auto_update: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self { ms_client_id: String::new(), launcher_update_url: String::new(), thm_manifest_url: String::new(),
            announcements_url: String::new(), java_path: String::new(), default_server: "play.6b6t.org".into(), slot_urls: HashMap::new(), auto_update: true }
    }
}
pub fn settings() -> Settings { load("settings.json") }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Account { pub id: String, pub kind: String, pub username: String, pub uuid: String, pub active: bool }

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Instance {
    pub id: String, pub name: String, pub kind: String, pub mc_version: String, pub loader: String,
    pub loader_version: Option<String>, pub game_dir: String, pub memory_mb: u32, pub jvm_args: String,
    pub java_path: String, pub width: u32, pub height: u32, pub fullscreen: bool, pub server: String,
    pub installed: bool, pub created_at: String,
}
impl Default for Instance {
    fn default() -> Self {
        Self { id: String::new(), name: String::new(), kind: "custom".into(), mc_version: String::new(), loader: "fabric".into(),
            loader_version: None, game_dir: String::new(), memory_mb: 4096, jvm_args: String::new(), java_path: String::new(),
            width: 854, height: 480, fullscreen: false, server: String::new(), installed: false, created_at: String::new() }
    }
}
pub fn instances() -> Vec<Instance> { load("instances.json") }
pub fn save_instances(v: &Vec<Instance>) -> R<()> { save("instances.json", v) }
pub fn instance(id: &str) -> R<Instance> { instances().into_iter().find(|i| i.id == id).ok_or_else(|| "instance not found".to_string()) }

pub fn activity(text: &str) {
    let mut v: Vec<serde_json::Value> = load("activity.json");
    v.insert(0, serde_json::json!({ "time": chrono::Utc::now().to_rfc3339(), "text": text }));
    v.truncate(30);
    save("activity.json", &v).ok();
}
