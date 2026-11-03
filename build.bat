@echo off
rem Reads your private signing key so the installer + update files get signed
if exist "%USERPROFILE%\.tauri\thm-launcher.key" set /p TAURI_SIGNING_PRIVATE_KEY=<"%USERPROFILE%\.tauri\thm-launcher.key"
set TAURI_SIGNING_PRIVATE_KEY_PASSWORD=
call npm install || exit /b 1
call npm run icons || exit /b 1
call npm run tauri build
pause
