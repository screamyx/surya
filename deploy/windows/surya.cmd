@echo off
rem surya launcher for Windows. Double-click it, or run it with the same flags
rem as zeron.exe (for example --engine ws://host:27700 --engine-token ...).
rem
rem Settings live under %APPDATA%\surya:
rem   servers.json   {"engine":"ws://host:27700","token":"..."}  (optional; the
rem                  Servers page inside the app is the normal way)
rem   data\          the app's data dir (ui-settings.json, logs\)
rem
rem zeron.exe is a console program. It runs under a headless console host so
rem no black console window appears; its log is data\logs\zeron-headed.log.
setlocal
if not defined HOME set "HOME=%USERPROFILE%"
set "SURYA_HOME=%APPDATA%\surya"
set "ZERON_DATA_DIR=%SURYA_HOME%\data"
if not exist "%ZERON_DATA_DIR%" mkdir "%ZERON_DATA_DIR%"
set "SURYA_EXTRA_ARGS=%*"
set "SURYA_EXE=%~dp0zeron.exe"
powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  "$a = @('--headless', $env:SURYA_EXE); $cfg = Join-Path $env:SURYA_HOME 'servers.json';" ^
  "if (Test-Path $cfg) { $j = Get-Content -Raw $cfg | ConvertFrom-Json; if ($j.engine) { $a += @('--engine', $j.engine); if ($j.token) { $a += @('--engine-token', $j.token) } } };" ^
  "if ($env:SURYA_EXTRA_ARGS) { $a += ($env:SURYA_EXTRA_ARGS -split ' ' | Where-Object { $_ }) };" ^
  "Start-Process -FilePath conhost.exe -ArgumentList $a -WindowStyle Hidden"
endlocal
