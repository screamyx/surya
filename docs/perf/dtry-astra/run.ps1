# Interactive scheduled task. Own paths only; no other seat's app is stopped.
param([Parameter(Mandatory=$true)][string]$Name, [int]$Seconds = 35)
$ErrorActionPreference = 'Stop'
$runs = 'E:\surya-astra-runs'
$data = 'E:\surya-astra-data'
New-Item -ItemType Directory -Force -Path $runs, $data | Out-Null
Copy-Item E:\surya-remote-ui-settings-r5.json "$data\ui-settings.json" -Force
$env:ZERON_DATA_DIR = $data
$env:ZERON_OPEN_PANE = 'browser'
$env:SURYA_BROWSER_URL = 'https://example.com'
$env:SURYA_CEF_CACHE = "$data\cef"
$env:SURYA_CEF_GPU = '1'
$env:RUST_LOG = 'info'
foreach ($key in 'ZERON_ENGINE', 'ZERON_ENGINE_TOKEN', 'SURYA_CEF_THREADED', 'SURYA_BROWSER_ZERO_COPY', 'SURYA_BROWSER_ZERO_COPY_PROBE', 'SURYA_SELFTEST_SCROLL', 'SURYA_SELFTEST_ANIM', 'SURYA_SELFTEST_TIMER', 'SURYA_PUMP_TIMER', 'SURYA_FRAME_LATENCY', 'SURYA_CEF_FPS', 'SURYA_PRESENT_WAITABLE', 'SURYA_EXTERNAL_BEGIN_FRAME', 'SURYA_COARSE_TIMER', 'SURYA_PUMP_MS', 'SURYA_ASTRA_BUILD') {
  Remove-Item "Env:$key" -ErrorAction SilentlyContinue
}
foreach ($line in Get-Content "$runs\$Name.env") {
  if ($line -match '^([A-Z_]+)=(.*)$') { Set-Item "Env:$($Matches[1])" $Matches[2] }
}
$build = $env:SURYA_ASTRA_BUILD
if (-not $build) { $build = 'working' }
if ($build -notin @('working', 'threaded', 'latency', 'present', 'combined', 'instrumented')) { throw "Unknown Astra build: $build" }
$exe = if ($build -eq 'working') { 'E:\surya-astra-target\release\zeron.exe' } else { "E:\surya-astra-bin-$build\zeron.exe" }
if (-not (Test-Path $exe)) { throw "Missing Astra executable: $exe" }
$helper = Join-Path (Split-Path $exe) 'zeron-browser-helper.exe'
$shaFile = if ($build -eq 'working') { 'E:\surya-astra\SHA' } else { Join-Path (Split-Path $exe) 'SHA' }
$binarySha = (Get-Content $shaFile).Trim()
$ours = @(Get-Process zeron, zeron-browser-helper -ErrorAction SilentlyContinue | Where-Object { $_.Path -like 'E:\surya-astra-*' })
if ($ours.Count) { throw 'A prior Astra run still owns a window' }
"start=$(Get-Date -Format o) exe=$exe sha=$binarySha seconds=$Seconds" | Out-File -Encoding utf8 "$runs\$Name.launch.log"
$p = Start-Process conhost.exe -ArgumentList @('--headless', $exe) -WindowStyle Hidden -RedirectStandardOutput "$runs\$Name.out.log" -RedirectStandardError "$runs\$Name.err.log" -PassThru
Start-Sleep -Seconds $Seconds
$ours = @(Get-Process zeron -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $exe })
foreach ($app in $ours) { $app.CloseMainWindow() | Out-Null }
Start-Sleep -Seconds 6
$remaining = @(Get-Process zeron, zeron-browser-helper -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $exe -or $_.Path -eq $helper })
"close_asked=$($ours.Count) close_remaining=$($remaining.Count)" | Out-File -Append -Encoding utf8 "$runs\$Name.launch.log"
foreach ($app in $remaining) { Stop-Process -Id $app.Id -Force }
"stopped=$(Get-Date -Format o)" | Out-File -Append -Encoding utf8 "$runs\$Name.launch.log"
