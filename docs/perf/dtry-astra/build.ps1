# Invoked over SSH by build.sh while its Linux cargo slot is held.
param([switch]$Seed)
$ErrorActionPreference = 'Stop'
$root = 'E:\surya-astra'
$log = 'E:\surya-astra-build.log'
$env:CARGO_TARGET_DIR = 'E:\surya-astra-target'
$env:CARGO_HOME = 'E:\surya-astra-cargo'
$env:CEF_PATH = 'E:\surya-cef2-cef'
$env:CARGO_BUILD_JOBS = '3'
if ($Seed) {
  foreach ($part in 'registry', 'git') {
    robocopy "E:\surya-gpu-cargo\$part" "$env:CARGO_HOME\$part" /E /MT:4 /NFL /NDL /NJH /NJS /NP | Out-Null
    if ($LASTEXITCODE -ge 8) { throw "Cargo cache copy failed: $LASTEXITCODE" }
  }
}
Set-Location "$root\app"
$env:ZERON_BUILD_SHA = (Get-Content "$root\SHA").Trim()
$env:ZERON_BUILD_COMMIT_TIME = [string][DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
"start=$(Get-Date -Format o) sha=$env:ZERON_BUILD_SHA jobs=$env:CARGO_BUILD_JOBS" | Out-File -Encoding utf8 $log
# The outer Linux wrapper owns a swarm slot for this complete SSH command.
Get-Command cargo -ErrorAction Stop | Out-Null
$ErrorActionPreference = 'Continue'
& cargo --version 2>&1 | Out-File -Append -Encoding utf8 $log
& cargo build --release -p zeron --features browser 2>&1 | Out-File -Append -Encoding utf8 $log
$result = $LASTEXITCODE
"BUILD_EXIT=$result end=$(Get-Date -Format o)" | Out-File -Append -Encoding utf8 $log
exit $result
