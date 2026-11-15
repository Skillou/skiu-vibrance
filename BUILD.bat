@echo off
chcp 65001 >nul
setlocal
cd /d "%~dp0"
echo.
echo  === Skiu Vibrance : build ===
echo.
where node >nul 2>nul || (echo [ERREUR] Node.js introuvable. Installe la version LTS : https://nodejs.org & pause & exit /b 1)
where cargo >nul 2>nul || (echo [ERREUR] Rust introuvable. Installe-le : https://rustup.rs & pause & exit /b 1)

rem Signature des mises a jour : cle du projet si elle est presente sur cette machine.
set "KEY=%USERPROFILE%\.tauri\skiu-vibrance.key"
set "EXTRA="
if not defined TAURI_SIGNING_PRIVATE_KEY if exist "%KEY%" (
  set "TAURI_SIGNING_PRIVATE_KEY=%KEY%"
  if exist "%KEY%.password" set /p TAURI_SIGNING_PRIVATE_KEY_PASSWORD=<"%KEY%.password"
)
if not defined TAURI_SIGNING_PRIVATE_KEY (
  echo [INFO] Cle de signature absente : installateur seul, sans fichiers de mise a jour.
  set "EXTRA=-- --config src-tauri/tauri.unsigned.conf.json"
)

echo [1/2] Installation des dependances...
call npm install --no-audit --no-fund || (echo [ERREUR] npm install a echoue & pause & exit /b 1)
echo [2/2] Compilation et installateur (la premiere fois : 5 a 10 minutes)...
call npm run tauri build %EXTRA% || (echo [ERREUR] La compilation a echoue. Copie le message ci-dessus. & pause & exit /b 1)
echo.
echo  Termine. Installateur dans :
echo  src-tauri\target\release\bundle\nsis
start "" "src-tauri\target\release\bundle\nsis"
pause
