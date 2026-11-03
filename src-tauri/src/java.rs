use crate::net::{progress, HTTP};
use crate::store::{self, Instance, R, S};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tauri::AppHandle;
use walkdir::WalkDir;

fn exe() -> &'static str { if cfg!(windows) { "java.exe" } else { "java" } }

fn find_in(dir: &std::path::Path) -> Option<PathBuf> {
    WalkDir::new(dir).max_depth(6).into_iter().filter_map(|e| e.ok())
        .find(|e| e.file_name() == exe() && e.path().parent().map(|p| p.ends_with("bin")).unwrap_or(false))
        .map(|e| e.into_path())
}

/// Resolution order: instance override -> global override -> managed runtime -> download Temurin JRE (SHA-256 verified).
pub async fn ensure(app: &AppHandle, inst: &Instance, major: u64) -> R<PathBuf> {
    for p in [inst.java_path.clone(), store::settings().java_path] {
        if !p.trim().is_empty() {
            let pb = PathBuf::from(p.trim());
            return if pb.exists() { Ok(pb) } else { Err(format!("Configured Java not found: {}", pb.display())) };
        }
    }
    let rt = store::dir().join("runtimes").join(format!("java-{major}"));
    if let Some(p) = find_in(&rt) { return Ok(p); }

    let os = if cfg!(windows) { "windows" } else if cfg!(target_os = "macos") { "mac" } else { "linux" };
    let arch = if cfg!(target_arch = "aarch64") { "aarch64" } else { "x64" };
    progress(app, &inst.id, &format!("Downloading Java {major}"), 0, 1);
    let url = format!("https://api.adoptium.net/v3/assets/latest/{major}/hotspot?architecture={arch}&image_type=jre&os={os}&vendor=eclipse");
    let meta = crate::net::get_json(&url).await?;
    let pkg = meta.pointer("/0/binary/package").ok_or("No Java runtime available for this platform")?;
    let link = pkg["link"].as_str().ok_or("bad Java metadata")?.to_string();
    let want = pkg["checksum"].as_str().ok_or("bad Java metadata")?.to_string();
    let bytes = HTTP.get(&link).send().await.s()?.error_for_status().s()?.bytes().await.s()?;
    if !hex::encode(Sha256::digest(&bytes)).eq_ignore_ascii_case(&want) { return Err("Java download checksum mismatch".into()); }
    let rt2 = rt.clone();
    let is_zip = link.ends_with(".zip");
    tokio::task::spawn_blocking(move || -> R<()> {
        std::fs::create_dir_all(&rt2).s()?;
        if is_zip {
            let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).s()?;
            for i in 0..z.len() {
                let mut f = z.by_index(i).s()?;
                let Some(rel) = f.enclosed_name() else { continue };
                let out = rt2.join(rel);
                if f.is_dir() { std::fs::create_dir_all(&out).s()?; } else {
                    if let Some(p) = out.parent() { std::fs::create_dir_all(p).s()?; }
                    std::io::copy(&mut f, &mut std::fs::File::create(&out).s()?).s()?;
                }
            }
        } else {
            let gz = flate2::read::GzDecoder::new(std::io::Cursor::new(bytes));
            tar::Archive::new(gz).unpack(&rt2).s()?;
        }
        Ok(())
    }).await.s()??;
    progress(app, &inst.id, &format!("Java {major} ready"), 1, 1);
    find_in(&rt).ok_or_else(|| "Java extracted but java executable not found".to_string())
}
