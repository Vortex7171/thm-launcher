use crate::store::{self, R, S};
use crate::{accounts, mc};
use once_cell::sync::Lazy;
use serde_json::json;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::oneshot;

static RUNNING: Lazy<Mutex<HashMap<String, oneshot::Sender<()>>>> = Lazy::new(|| Mutex::new(HashMap::new()));

#[tauri::command]
pub async fn install_instance(app: AppHandle, id: String) -> R<()> {
    let inst = store::instance(&id)?;
    let (_vid, lv) = mc::install(&app, &inst).await?;
    if inst.kind == "thm" { crate::thm::install_missing(&app, &inst).await?; }
    let mut all = store::instances();
    if let Some(i) = all.iter_mut().find(|i| i.id == id) { i.installed = true; if lv.is_some() { i.loader_version = lv; } }
    store::save_instances(&all)?;
    store::activity(&format!("Installed {} ({})", inst.name, inst.mc_version));
    Ok(())
}

#[tauri::command]
pub async fn launch_instance(app: AppHandle, id: String) -> R<()> {
    if RUNNING.lock().unwrap().contains_key(&id) { return Err("Instance is already running".into()); }
    let inst = store::instance(&id)?;
    if !inst.installed { return Err("Instance is not installed yet".into()); }
    let acct = accounts::active()?;
    let sess = accounts::session(&acct).await?;
    let mut major = 8;
    let cmd = mc::build_command(&inst, &sess, &mut major)?;
    let java = crate::java::ensure(&app, &inst, major).await?;

    let mut c = tokio::process::Command::new(&java);
    c.args(&cmd.args).current_dir(&cmd.dir).stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::null()).kill_on_drop(true);
    #[cfg(windows)] c.creation_flags(0x08000000);
    let mut child = c.spawn().map_err(|e| format!("Failed to start Java: {e}"))?;
    let (tx, rx) = oneshot::channel::<()>();
    RUNNING.lock().unwrap().insert(id.clone(), tx);
    store::activity(&format!("Launched {} as {}", inst.name, sess.name));
    app.emit("game-started", json!({ "id": id })).ok();

    let token = sess.token.clone();
    let pipe = |app: AppHandle, id: String, token: String, r: Box<dyn tokio::io::AsyncRead + Unpin + Send>| {
        tokio::spawn(async move {
            let mut l = BufReader::new(r).lines();
            while let Ok(Some(line)) = l.next_line().await {
                let line = if token.len() > 8 { line.replace(&token, "***") } else { line };
                app.emit("game-log", json!({ "id": id, "line": line })).ok();
            }
        });
    };
    if let Some(o) = child.stdout.take() { pipe(app.clone(), id.clone(), token.clone(), Box::new(o)); }
    if let Some(e) = child.stderr.take() { pipe(app.clone(), id.clone(), token.clone(), Box::new(e)); }

    let game_dir = inst.game_dir.clone();
    tokio::spawn(async move {
        let mut killed = false;
        let code = tokio::select! {
            st = child.wait() => st.ok().and_then(|s| s.code()),
            _ = rx => { killed = true; child.kill().await.ok(); child.wait().await.ok().and_then(|s| s.code()) }
        };
        RUNNING.lock().unwrap().remove(&id);
        let crashed = !killed && code.map(|c| c != 0).unwrap_or(true);
        let report = std::fs::read_dir(std::path::Path::new(&game_dir).join("crash-reports")).ok().and_then(|d| d.filter_map(|e| e.ok()).max_by_key(|e| e.metadata().and_then(|m| m.modified()).ok())).map(|e| e.path().display().to_string());
        app.emit("game-exit", json!({ "id": id, "code": code, "crashed": crashed, "report": if crashed { report } else { None } })).ok();
    });
    Ok(())
}

#[tauri::command]
pub fn kill_instance(id: String) -> R<()> {
    if let Some(tx) = RUNNING.lock().unwrap().remove(&id) { tx.send(()).ok(); }
    Ok(())
}
#[tauri::command]
pub fn list_running() -> Vec<String> { RUNNING.lock().unwrap().keys().cloned().collect() }
pub fn is_running(id: &str) -> bool { RUNNING.lock().unwrap().contains_key(id) }
