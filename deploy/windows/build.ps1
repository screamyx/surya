<#
Build the surya Windows app and pack it as a folder plus a zip.

  powershell -ExecutionPolicy Bypass -File deploy\windows\build.ps1
  ... -NoBuild            reuse target\release\zeron.exe
  ... -Sha abc1234        version tag when the checkout has no .git
  ... -Browser            build with the CEF browser pane (cargo feature
                          `browser`) and ship Chromium's runtime files and
                          zeron-browser-helper.exe next to the exe.
                          -NoBuild -Browser packs an earlier -Browser build;
                          -NoBuild without -Browser refuses a release dir
                          that holds a browser build (its exe needs libcef).

Needs: Rust (MSVC toolchain), Visual Studio Build Tools with the C++ workload.
Building with -Browser also needs CMake, Ninja (`python -m pip install
ninja`) and CEF_PATH set to a directory the cef crate may download into
(about 250 MB once; app\crates\browser\README.md). -NoBuild reads none of
these.
Output: dist\surya-windows\ (zeron.exe, surya.cmd, VERSION.txt; with
        -Browser also zeron-browser-helper.exe, libcef.dll and the other
        CEF DLLs, *.pak, icudtl.dat, v8_context_snapshot.bin, locales\,
        CREDITS.html, CEF-LICENSE.txt, archive.json) and
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

if ($Browser -and -not $NoBuild -and -not $env:CEF_PATH) {
    throw "-Browser needs CEF_PATH (a directory the cef crate downloads CEF into, once); unset it would re-download into the build dir"
}
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
if ($NoBuild -and -not $Browser -and (Test-Path (Join-Path $target "release\libcef.dll"))) {
    throw "release dir holds a -Browser build (libcef.dll beside zeron.exe); its exe needs CEF, so pack it with -Browser or rebuild without -NoBuild"
}

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
# DLLs of the app's own travel with it, by name (none today; the fonts and
# icons are compiled into the binary). Never a *.dll sweep: after one
# -Browser build the release dir also holds Chromium's 300 MB, which a plain
# zip must not carry.
$release = Join-Path $target "release"
$appDlls = @()
foreach ($name in $appDlls) {
    $src = Join-Path $release $name
    if (-not (Test-Path $src)) { throw "missing $src" }
    Copy-Item $src $dist
}
Copy-Item (Join-Path $PSScriptRoot "surya.cmd") (Join-Path $dist "surya.cmd")
$cefVersion = $null
if ($Browser) {
    # The cef crate's build script copied Chromium's runtime files next to
    # the exe in target\release; CEF loads them from the exe's own folder, so
    # the zip carries the same set. Every name is required: a missing one is
    # a Chromium that fails at start-up, not a smaller zip. CREDITS.html is
    # Chromium's third-party notices (from the CEF dist root); archive.json
    # names the exact CEF + Chromium build that was downloaded.
    $cefRequired = @(
        "zeron-browser-helper.exe",
        "libcef.dll", "chrome_elf.dll",
        "d3dcompiler_47.dll", "dxcompiler.dll", "dxil.dll",
        "libEGL.dll", "libGLESv2.dll",
        "vk_swiftshader.dll", "vk_swiftshader_icd.json", "vulkan-1.dll",
        "chrome_100_percent.pak", "chrome_200_percent.pak", "resources.pak",
        "icudtl.dat", "v8_context_snapshot.bin",
        "CREDITS.html", "archive.json"
    )
    $missing = @($cefRequired | Where-Object { -not (Test-Path (Join-Path $release $_)) })
    if ($missing.Count -gt 0) {
        throw "browser runtime incomplete in ${release}: missing $($missing -join ', ') (build with -Browser, not -NoBuild from a plain build)"
    }
    $locales = Join-Path $release "locales"
    $localeCount = @(Get-ChildItem $locales -Filter *.pak -File -ErrorAction SilentlyContinue).Count
    if ($localeCount -lt 1) { throw "browser runtime incomplete: no *.pak in $locales" }
    foreach ($name in $cefRequired) { Copy-Item (Join-Path $release $name) $dist -Force }
    $distLocales = Join-Path $dist "locales"
    New-Item -ItemType Directory -Force -Path $distLocales | Out-Null
    Get-ChildItem $locales -File | ForEach-Object { Copy-Item $_.FullName $distLocales -Force }
    Copy-Item (Join-Path $PSScriptRoot "..\CEF-LICENSE.txt") $dist
    $m = [regex]::Match((Get-Content (Join-Path $release "archive.json") -Raw), 'cef_binary_([^+]+)\+g[0-9a-f]+\+chromium-([0-9.]+)')
    if (-not $m.Success) { throw "archive.json beside the exe does not name a cef_binary_<cef>+g<hash>+chromium-<version> archive" }
    $cefVersion = "CEF $($m.Groups[1].Value), Chromium $($m.Groups[2].Value)"
    Write-Host "== browser: $cefVersion; shipped $($cefRequired.Count) required files, $localeCount locales, CEF-LICENSE.txt"
}
@(
    "surya windows app",
    "commit: $Sha",
    "built: $(Get-Date -Format o) on $env:COMPUTERNAME",
    "run: surya.cmd (see docs\quickstart.md)",
    "browser pane: $(if ($Browser) { "yes, $cefVersion, BSD-3-Clause (CEF-LICENSE.txt, CREDITS.html)" } else { 'no' })"
) | Out-File -Encoding utf8 (Join-Path $dist "VERSION.txt")

$zip = Join-Path $Root "dist\surya-windows-$Sha.zip"
if (Test-Path $zip) { Remove-Item -Force $zip }
Compress-Archive -Path (Join-Path $dist "*") -DestinationPath $zip
Write-Host "== done"
Write-Host "  folder: $dist"
Write-Host "  zip:    $zip"
