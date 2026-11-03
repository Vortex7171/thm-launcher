# THM Launcher

A black-and-white Minecraft launcher for [The Highway Men](https://highwaymen.cc) and 6b6t. Sign in, press Play, and the THM setup (Fabric, AnarchyMod, Meteor, Baritone, THM Addons) installs itself.

## Download (Windows)

**[Click here for the latest version](https://github.com/Vortex7171/thm-launcher/releases/latest)**

1. Open the link and scroll down to **Assets**.
2. Click the file ending in **`x64-setup.exe`** and let it download.
3. Run it and click through the installer.
4. Open **THM Launcher** from the Start Menu.

Windows may say "Windows protected your PC". The installer isn't signed with a paid certificate yet. Click **More info**, then **Run anyway**.

After the first install, the launcher updates itself. You can turn that off in **Settings**.

## Features
- Microsoft accounts and offline profiles, with easy switching
- Ready-to-use THM instance on Minecraft 1.21.11 with Fabric
- Multiple instances: create, clone, rename, delete, repair, import, export
- Mod manager: import JARs, enable or disable, duplicate and missing-dependency warnings, Modrinth search and updates
- Automatic Java download, live game log, crash detection
- 6b6t server status and quick connect
- In-app launcher updates (signed)

## Problems?
Open an [issue](https://github.com/Vortex7171/thm-launcher/issues) and say what happened. Early version, so bugs are expected.

---

## For developers

**Run while editing:** `npm install`, `npm run icons`, `npm run tauri dev`

**Build an installer locally:** double-click `build.bat`. The installer is in `src-tauri/target/release/bundle/nsis/`.

**One-time updater setup**
1. `npm run tauri signer generate -- -w "%USERPROFILE%\.tauri\thm-launcher.key"` (blank password). Back the key up privately and never commit it.
2. `npm run setup-updater` (asks for `username/repo`).
3. Add the repo secret `TAURI_SIGNING_PRIVATE_KEY` (contents of the `.key` file) and set Actions workflow permissions to read and write.

**Release an update**
1. `npm run release -- 0.1.1`
2. `git add .`, `git commit -m "v0.1.1"`, `git tag v0.1.1`, `git push`, `git push --tags`
3. GitHub builds, signs and publishes the release. Installed launchers pick it up automatically.

Component download links are in `src-tauri/thm-manifest.json`.