@echo off
cd /d "%~dp0"
call npm install --no-audit --no-fund
call npm run tauri dev
