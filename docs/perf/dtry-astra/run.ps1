# Interactive scheduled task. Own paths only; no other seat's app is stopped.
param([Parameter(Mandatory=$true)][string]$Name, [int]$Seconds = 35)
$ErrorActionPreference = 'Stop'
$runs = 'E:\surya-astra-runs'
$data = 'E:\surya-astra-data'
$exe = 'E:\surya-astra-target\release\zeron.exe'
New-Item -ItemType Directory -Force -Path $runs, $data | Out-Null
Copy-Item E:\surya-remote-ui-settings-r5.json "$data\ui-settings.json" -Force
$env:ZERON_DATA_DIR = $data
$env:ZERON_OPEN_PANE = 'browser'
$env:SURYA_BROWSER_URL = 'https://example.com'
$env:SURYA_CEF_CACHE = "$data\cef"
$env:SURYA_CEF_GPU = '1'
$env:RUST_LOG = 'info'
foreach ($key in 'ZERON_ENGINE', 'ZERON_ENGINE_TOKEN', 'SURYA_CEF_THREADED', 'SURYA_BROWSER_ZERO_COPY', 'SURYA_BROWSER_ZERO_COPY_PROBE', 'SURYA_SELFTEST_SCROLL', 'SURYA_SELFTEST_ANIM', 'SURYA_SELFTEST_TIMER', 'SURYA_PUMP_TIMER', 'SURYA_FRAME_LATENCY', 'SURYA_CEF_FPS') {
  Remove-Item "Env:$key" -ErrorAction SilentlyContinue
}
foreach ($line in Get-Content "$runs\$Name.env") {
  if ($line -match '^([A-Z_]+)=(.*)$') { Set-Item "Env:$($Matches[1])" $Matches[2] }
}
$ours = @(Get-Process zeron -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $exe })
if ($ours.Count) { throw 'A prior Astra run still owns a window' }
"start=$(Get-Date -Format o) exe=$exe seconds=$Seconds" | Out-File -Encoding utf8 "$runs\$Name.launch.log"
$p = Start-Process conhost.exe -ArgumentList @('--headless', $exe) -WindowStyle Hidden -RedirectStandardOutput "$runs\$Name.out.log" -RedirectStandardError "$runs\$Name.err.log" -PassThru
Start-Sleep -Seconds $Seconds
$ours = @(Get-Process zeron -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $exe })
foreach ($app in $ours) { $app.CloseMainWindow() | Out-Null }
Start-Sleep -Seconds 6
$remaining = @(Get-Process zeron -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $exe })
"close_asked=$($ours.Count) close_remaining=$($remaining.Count)" | Out-File -Append -Encoding utf8 "$runs\$Name.launch.log"
foreach ($app in $remaining) { Stop-Process -Id $app.Id -Force }
"stopped=$(Get-Date -Format o)" | Out-File -Append -Encoding utf8 "$runs\$Name.launch.log"
