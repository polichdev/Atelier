@echo off
setlocal
cd /d "%~dp0.."

powershell -NoProfile -Command "$paths = @((Join-Path (Get-Location) 'target\debug\Atelier.exe'), (Join-Path (Get-Location) 'target\dev-run\Atelier.exe')); $guardians = @(Get-CimInstance Win32_Process -Filter ('Name = '+[char]39+'Atelier.exe'+[char]39) | Where-Object { ($_.CommandLine -like '*--dock-watchdog*') -or ($_.CommandLine -like '*--native-taskbar-guardian*') } | Select-Object -ExpandProperty ProcessId); Get-Process -Name Atelier -ErrorAction SilentlyContinue | Where-Object { ($paths -contains $_.Path) -and ($guardians -notcontains $_.Id) } | Stop-Process; Start-Sleep -Milliseconds 500"
if errorlevel 1 exit /b 1

cargo build -p atelier-ui --bin Atelier
if errorlevel 1 exit /b 1

if not exist "target\dev-run" mkdir "target\dev-run"
if errorlevel 1 exit /b 1

copy /Y "target\debug\Atelier.exe" "target\dev-run\Atelier.exe" >nul
if errorlevel 1 (
    echo Could not update the development copy. Close the running Atelier development app and try again.
    exit /b 1
)

"target\dev-run\Atelier.exe"
