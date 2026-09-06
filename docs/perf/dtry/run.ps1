# One measurement run on dtry, session 1: launch surya with the browser pane
# open and one self-test armed, wait, stop it. The self-test's own line
# lands in E:\surya-perf-runs\<name>.out.log.
#
# Written for a scheduled task (schtasks /it); the environment for the run
# comes from E:\surya-perf-runs\<name>.env, one KEY=VALUE per line, which
# run.sh writes. The task name is the run name.
#
#   run.ps1 -Name scroll-pool -Seconds 40
param([string]$Name, [int]$Seconds = 40, [string]$Exe = 'E:\surya-perf-target\release\surya.exe')
$ErrorActionPreference = 'Continue'
$runs = 'E:\surya-perf-runs'
$data = 'E:\surya-perf-data'
New-Item -ItemType Directory -Force -Path $runs, $data | Out-Null
# The same seeded settings cef2 launched the pane with: the service engine
# and a browser pane open, no onboarding in the way.
Copy-Item E:\surya-remote-ui-settings-r5.json "$data\ui-settings.json" -Force
$env:SURYA_DATA_DIR = $data
$env:SURYA_OPEN_PANE = 'browser'
$env:SURYA_BROWSER_URL = 'https://example.com'
$env:SURYA_CEF_CACHE = "$data\cef"
$env:RUST_LOG = 'info'
$env:HOME = $env:USERPROFILE
Remove-Item Env:SURYA_ENGINE -ErrorAction SilentlyContinue
Remove-Item Env:SURYA_ENGINE_TOKEN -ErrorAction SilentlyContinue
foreach ($v in 'SURYA_PUMP_TIMER', 'SURYA_PUMP_MS', 'SURYA_CEF_FPS', 'SURYA_COARSE_TIMER', 'SURYA_SELFTEST_SCROLL', 'SURYA_SELFTEST_ANIM', 'SURYA_SELFTEST_TIMER', 'SURYA_FRAME_LATENCY', 'SURYA_BROWSER_ZERO_COPY') {
  Remove-Item "Env:$v" -ErrorAction SilentlyContinue
}
$envfile = "$runs\$Name.env"
if (Test-Path $envfile) {
  foreach ($line in Get-Content $envfile) {
    if ($line -match '^([A-Z_]+)=(.*)$') { Set-Item "Env:$($Matches[1])" $Matches[2] }
  }
}
$out = "$runs\$Name.out.log"
$err = "$runs\$Name.err.log"
"run $Name $(Get-Date -Format o) exe=$Exe seconds=$Seconds env=$(Get-Content $envfile -ErrorAction SilentlyContinue -Raw)" | Out-File -Encoding utf8 "$runs\$Name.launch.log"
Get-Process surya -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 2
$p = Start-Process -FilePath conhost.exe -ArgumentList @('--headless', $Exe) -WindowStyle Hidden -RedirectStandardOutput $out -RedirectStandardError $err -PassThru
Start-Sleep -Seconds $Seconds
# Idle cost over the last part of a still-page run: the process with the
# window, its CPU time across a window, as haktui's measure-threads.ps1 does.
if ($env:SURYA_PERF_IDLE_SECONDS) {
  $z = Get-Process surya -ErrorAction SilentlyContinue | Sort-Object WorkingSet64 -Descending | Select-Object -First 1
  if ($z) {
    $cpu0 = $z.TotalProcessorTime.TotalMilliseconds
    $secs = [int]$env:SURYA_PERF_IDLE_SECONDS
    Start-Sleep -Seconds $secs
    $z.Refresh()
    $cpu1 = $z.TotalProcessorTime.TotalMilliseconds
    "idle: pid=$($z.Id) cpu_ms=$([math]::Round($cpu1-$cpu0)) over ${secs}s = $([math]::Round(($cpu1-$cpu0)/($secs*10),1))% of one core" | Out-File -Append -Encoding utf8 "$runs\$Name.launch.log"
  }
}
Get-Process surya -ErrorAction SilentlyContinue | Stop-Process -Force
"stopped $(Get-Date -Format o)" | Out-File -Append -Encoding utf8 "$runs\$Name.launch.log"
