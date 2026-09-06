# Release build of surya with the browser feature on dtry, for the frame
# rate measurements. Runs as a scheduled task so it survives the SSH session.
#
#   ssh dtry 'schtasks /create /tn surya-perf-build /tr "powershell.exe -NoProfile -ExecutionPolicy Bypass -File E:\surya-perf\docs\perf\dtry\build.ps1" /sc once /st 00:00 /f; schtasks /run /tn surya-perf-build'
#
# Reads: E:\surya-perf\app (the tree), E:\surya-cef2-cef (CEF 151.3.24 for
# Windows, shared, read-only), E:\surya-perf-cargo (own cargo home).
# Writes: E:\surya-perf-target\release\surya.exe, E:\surya-perf-build.log.
$ErrorActionPreference = 'Continue'
$log = 'E:\surya-perf-build.log'
$sha = if ($env:SURYA_BUILD_SHA) { $env:SURYA_BUILD_SHA } else { Get-Content E:\surya-perf\SHA -ErrorAction SilentlyContinue }
"start $(Get-Date -Format o) sha=$sha profile=release" | Out-File -Encoding utf8 $log
$env:CARGO_TARGET_DIR = 'E:\surya-perf-target'
$env:CARGO_HOME = 'E:\surya-perf-cargo'
$env:CEF_PATH = 'E:\surya-cef2-cef'
$env:HOME = $env:USERPROFILE
$env:SURYA_BUILD_SHA = $sha
$env:SURYA_BUILD_COMMIT_TIME = [string][int][double]::Parse((Get-Date -UFormat %s))
Set-Location E:\surya-perf\app
cargo --version 2>&1 | Out-File -Append -Encoding utf8 $log
cargo build --release -p surya --features browser 2>&1 | Out-File -Append -Encoding utf8 $log
"exit=$LASTEXITCODE end $(Get-Date -Format o)" | Out-File -Append -Encoding utf8 $log
