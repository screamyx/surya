<#
Build the surya Windows app and pack it as a folder plus a zip.

  powershell -ExecutionPolicy Bypass -File deploy\windows\build.ps1
  ... -NoBuild            reuse target\release\zeron.exe
  ... -Sha abc1234        version tag when the checkout has no .git
  ... -Browser            build with the CEF browser pane (cargo feature
                          `browser`) and ship Chromium's runtime files and
                          zeron-browser-helper.exe next to the exe

Needs: Rust (MSVC toolchain), Visual Studio Build Tools with the C++ workload.
With -Browser also: CMake, Ninja (`python -m pip install ninja`), and
CEF_PATH pointing at a directory the cef crate may download into (about
250 MB once; app\crates\browser\README.md).
Output: dist\surya-windows\ (zeron.exe, surya.cmd, VERSION.txt) and
        dist\surya-windows-<sha>.zip
#>
param(
    [switch]$NoBuild,
    [switch]$Browser,
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
        if ($Browser) {
            cargo build --release -p zeron --features browser
        } else {
            cargo build --release -p zeron
        }
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
if ($Browser) {
    # The cef crate's build script copied Chromium's runtime files (libcef.dll,
    # chrome_elf.dll, *.pak, icudtl.dat, *.bin, locales\) next to the exe in
    # target\release; CEF loads them from the exe's own folder, so the zip
    # carries the same set. The helper is CEF's subprocess (renderer, GPU).
    $release = Join-Path $target "release"
    $helper = Join-Path $release "zeron-browser-helper.exe"
    if (-not (Test-Path $helper)) { throw "no $helper (build with -Browser, not -NoBuild from a plain build)" }
    if (-not (Test-Path (Join-Path $release "libcef.dll"))) { throw "no libcef.dll in $release (the cef build script did not run)" }
    Copy-Item $helper $dist
    $cefFiles = @("*.dll", "*.pak", "*.dat", "*.bin", "*.json")
    Get-ChildItem (Join-Path $release "*") -File -Include $cefFiles |
        ForEach-Object { Copy-Item $_.FullName $dist -Force }
    Copy-Item (Join-Path $release "locales") (Join-Path $dist "locales") -Recurse -Force
    $shipped = @(Get-ChildItem $dist -File).Count
    Write-Host "== browser: shipped $shipped files (helper + CEF runtime) and locales\"
}
@(
    "surya windows app",
    "commit: $Sha",
    "built: $(Get-Date -Format o) on $env:COMPUTERNAME",
    "run: surya.cmd (see docs\quickstart.md)",
    "browser pane: $(if ($Browser) { 'yes (CEF)' } else { 'no' })"
) | Out-File -Encoding utf8 (Join-Path $dist "VERSION.txt")

$zip = Join-Path $Root "dist\surya-windows-$Sha.zip"
if (Test-Path $zip) { Remove-Item -Force $zip }
Compress-Archive -Path (Join-Path $dist "*") -DestinationPath $zip
Write-Host "== done"
Write-Host "  folder: $dist"
Write-Host "  zip:    $zip"
