@echo off
rem surya launcher for Windows. Double-click it, or run it with the same flags
rem as zeron.exe (for example --engine ws://host:27700 --engine-token ...).
rem
rem Settings live under %APPDATA%\surya:
rem   servers.json   {"engine":"ws://host:27700","token":"..."}  (optional; the
rem                  Servers page inside the app is the normal way)
rem   data\          the app's data dir (ui-settings.json, logs\)
setlocal
if not defined HOME set "HOME=%USERPROFILE%"
set "SURYA_HOME=%APPDATA%\surya"
set "ZERON_DATA_DIR=%SURYA_HOME%\data"
if not exist "%ZERON_DATA_DIR%" mkdir "%ZERON_DATA_DIR%"
set "SURYA_EXTRA_ARGS=%*"
set "ENGINE_ARGS="
if exist "%SURYA_HOME%\servers.json" (
  for /f "usebackq delims=" %%L in (`powershell -NoProfile -Command "$j = Get-Content -Raw '%SURYA_HOME%\servers.json' | ConvertFrom-Json; if ($j.engine) { $a = '--engine ' + $j.engine; if ($j.token) { $a += ' --engine-token ' + $j.token }; $a }"`) do set "ENGINE_ARGS=%%L"
)
start "surya" "%~dp0zeron.exe" %ENGINE_ARGS% %*
endlocal
