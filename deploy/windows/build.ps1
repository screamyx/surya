<#
Build the surya Windows app and pack it as a folder plus a zip.

  powershell -ExecutionPolicy Bypass -File deploy\windows\build.ps1
  ... -NoBuild            reuse target\release\zeron.exe
  ... -Sha abc1234        version tag when the checkout has no .git

Needs: Rust (MSVC toolchain), Visual Studio Build Tools with the C++ workload.
Output: dist\surya-windows\ (zeron.exe, surya.cmd, VERSION.txt) and
        dist\surya-windows-<sha>.zip
#>
param(
    [switch]$NoBuild,
    [string]$Sha = "",
    [string]$CommitTime = "",
    [string]$Root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
)
$ErrorActionPreference = "Stop"
$app = Join-Path $Root "app"
$target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $app "target" }
$dist = Join-Path $Root "dist\surya-windows"
if (-not $env:HOME) { $env:HOME = $env:USERPROFILE }

if (-not $NoBuild) {
    # A tarball checkout has no .git: hand the stamp to crates/proto/build.rs.
    if ($Sha) { $env:ZERON_BUILD_SHA = $Sha }
    if ($CommitTime) { $env:ZERON_BUILD_COMMIT_TIME = $CommitTime }
    Write-Host "== building the app (release)"
    Push-Location $app
    try {
        cargo --version
        cargo build --release -p zeron
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed with exit $LASTEXITCODE" }
    } finally { Pop-Location }
}
$exe = Join-Path $target "release\zeron.exe"
if (-not (Test-Path $exe)) { throw "no binary at $exe (run without -NoBuild)" }

if (-not $Sha) {
    $Sha = "nogit-$(Get-Date -Format yyyyMMdd-HHmm)"
    if (Test-Path (Join-Path $Root ".git")) {
        $git = (git -C $Root rev-parse --short HEAD 2>$null)
        if ($git) { $Sha = $git.Trim() }
    }
}

Write-Host "== packing $dist"
if (Test-Path $dist) { Remove-Item -Recurse -Force $dist }
New-Item -ItemType Directory -Force -Path $dist | Out-Null
Copy-Item $exe (Join-Path $dist "zeron.exe")
# Any DLL the build placed beside the exe travels with it (none today; the
# fonts and icons are compiled into the binary).
Get-ChildItem (Join-Path $target "release") -Filter *.dll -ErrorAction SilentlyContinue |
    ForEach-Object { Copy-Item $_.FullName $dist }
Copy-Item (Join-Path $PSScriptRoot "surya.cmd") (Join-Path $dist "surya.cmd")
@(
    "surya windows app",
    "commit: $Sha",
    "built: $(Get-Date -Format o) on $env:COMPUTERNAME",
    "run: surya.cmd (see docs\quickstart.md)"
) | Out-File -Encoding utf8 (Join-Path $dist "VERSION.txt")

$zip = Join-Path $Root "dist\surya-windows-$Sha.zip"
if (Test-Path $zip) { Remove-Item -Force $zip }
Compress-Archive -Path (Join-Path $dist "*") -DestinationPath $zip
Write-Host "== done"
Write-Host "  folder: $dist"
Write-Host "  zip:    $zip"
