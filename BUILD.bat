@echo off
chcp 65001 >nul
setlocal
cd /d "%~dp0"
echo.
echo  === Skiu Vibrance : build ===
echo.
where node >nul 2>nul || (echo [ERREUR] Node.js introuvable. Installe la version LTS : https://nodejs.org & pause & exit /b 1)
where cargo >nul 2>nul || (echo [ERREUR] Rust introuvable. Installe-le : https://rustup.rs & pause & exit /b 1)
echo [1/2] Installation des dependances...
call npm install --no-audit --no-fund || (echo [ERREUR] npm install a echoue & pause & exit /b 1)
echo [2/2] Compilation et installateur (la premiere fois : 5 a 10 minutes)...
call npm run tauri build || (echo [ERREUR] La compilation a echoue. Copie le message ci-dessus. & pause & exit /b 1)
echo.
echo  Termine. Installateur dans :
echo  src-tauri\target\release\bundle\nsis
start "" "src-tauri\target\release\bundle\nsis"
pause
