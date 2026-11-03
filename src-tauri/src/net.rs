use crate::store::{R, S};
use futures::StreamExt;
use once_cell::sync::Lazy;
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

pub static HTTP: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder().user_agent("THM-Launcher/0.1 (+https://highwaymen.cc)")
        .timeout(std::time::Duration::from_secs(120)).build().unwrap()
});

pub async fn get_json(url: &str) -> R<Value> {
    HTTP.get(url).send().await.s()?.error_for_status().s()?.json().await.s()
}
pub fn sha1_hex(b: &[u8]) -> String { hex::encode(Sha1::digest(b)) }
pub fn progress(app: &AppHandle, id: &str, label: &str, done: usize, total: usize) {
    app.emit("progress", json!({ "id": id, "label": label, "done": done, "total": total })).ok();
}

#[derive(Clone)]
pub struct Job { pub url: String, pub dest: PathBuf, pub sha1: Option<String> }

/// Download with SHA-1 verification. Skips files already present and valid.
pub async fn download(url: &str, dest: &Path, sha1: Option<&str>) -> R<()> {
    if let Ok(b) = tokio::fs::read(dest).await {
        match sha1 { Some(h) => { if sha1_hex(&b).eq_ignore_ascii_case(h) { return Ok(()); } } None => return Ok(()) }
    }
    let bytes = HTTP.get(url).send().await.s()?.error_for_status().s()?.bytes().await.s()?;
    if let Some(h) = sha1 {
        if !sha1_hex(&bytes).eq_ignore_ascii_case(h) { return Err(format!("checksum mismatch for {url}")); }
    }
    if let Some(p) = dest.parent() { tokio::fs::create_dir_all(p).await.s()?; }
    let tmp = dest.with_extension("part");
    tokio::fs::write(&tmp, &bytes).await.s()?;
    tokio::fs::rename(&tmp, dest).await.s()
}

pub async fn download_all(app: &AppHandle, id: &str, label: &str, jobs: Vec<Job>) -> R<()> {
    let total = jobs.len();
    let done = Arc::new(AtomicUsize::new(0));
    let mut s = futures::stream::iter(jobs.into_iter().map(|j| {
        let (done, app, id, label) = (done.clone(), app.clone(), id.to_string(), label.to_string());
        async move {
            let mut last = String::new();
            for _ in 0..3 {
                match download(&j.url, &j.dest, j.sha1.as_deref()).await {
                    Ok(()) => { last.clear(); break; }
                    Err(e) => last = e,
                }
            }
            if !last.is_empty() { return Err(last); }
            let d = done.fetch_add(1, Ordering::SeqCst) + 1;
            if d % 8 == 0 || d == total { progress(&app, &id, &label, d, total); }
            Ok::<(), String>(())
        }
    })).buffer_unordered(16);
    while let Some(r) = s.next().await { r?; }
    Ok(())
}
