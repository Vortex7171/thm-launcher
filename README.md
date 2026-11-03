# THM Launcher

Black-and-white Minecraft launcher for The Highway Men (https://highwaymen.cc) and 6b6t. Tauri v2 + Rust backend, React + TypeScript frontend.

## Build (Windows installer)
1. Install Node 20+, Rust (rustup), and the Tauri prerequisites (WebView2 + MSVC build tools).
2. Double-click `build.bat`, or run: `npm install && npm run icons && npm run tauri build`
3. Installer: `src-tauri/target/release/bundle/nsis/*.exe` (also `.msi`).
No Windows machine? Push to GitHub — `.github/workflows/build.yml` builds Windows/macOS/Linux installers as artifacts.
Dev mode: `npm run icons && npm run tauri dev`.

## Notes
- **Microsoft login** needs no setup: click Sign in, enter the code at the page that opens. (It uses Mojang's own launcher client ID; an optional custom Azure ID can be set in Settings > Java & server.)
- **THM instance** is Minecraft 1.21.11 + Fabric. On first Play it downloads Fabric API, 6b6t AnarchyMod 1.5.0, Meteor Client, Baritone and THM Addons 0.2.9. URLs live in `src-tauri/thm-manifest.json`.
- Optional feeds in Settings: launcher update JSON, announcements JSON, THM manifest URL.
- Logo: replace `app-icon.png` (1024px) then `npm run icons`.

## Implemented
Microsoft device-code login (Xbox/XSTS/ownership check, refresh token in OS keychain), offline profiles, multi-account switching; instances (create/clone/rename/delete/repair/import/export, per-instance version, loader, mods, saves, Java, memory, JVM args); real Minecraft + Fabric install (SHA-1-verified libraries/assets), managed Java download (SHA-256-verified Temurin), launch with log streaming, kill and crash detection; mod list/toggle/delete/import/export, duplicate/missing-dependency/compat warnings, Modrinth search/install/update; 6b6t status (mcsrvstat.us); update checks with explicit confirmation.

## Status
**This source was written without a compiler or network access, so it has NOT been compiled or run.** Expect to fix a few compile errors on first `cargo build`. Offline-account launch is the best first test. Not implemented: modpack formats (.mrpack/CurseForge), Forge/NeoForge/Quilt, Minecraft versions older than ~1.6, self-installing launcher updates (it opens the download link instead).

## Auto-update (GitHub Releases)
1. `npm run tauri signer generate -- -w %USERPROFILE%\.tauri\thm-launcher.key` (blank password). Back the key up privately.
2. Create a public GitHub repo, then `npm run setup-updater` (asks for username/repo, fills in tauri.conf.json).
3. Add repo secret `TAURI_SIGNING_PRIVATE_KEY` = contents of the .key file.
4. Release: `npm run release -- 0.1.1`, then commit, tag `v0.1.1`, push with tags. GitHub builds and publishes; installed launchers update themselves.
