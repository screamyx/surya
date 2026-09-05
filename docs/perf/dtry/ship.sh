#!/usr/bin/env bash
# Ship this checkout's app/ and docs/perf/dtry to dtry as E:\surya-perf and
# touch every source there, so cargo never reuses a stale rlib (surya-remote's
# finding). Prints the sha it shipped.
#
#   docs/perf/dtry/ship.sh
set -eu
root="$(cd "$(dirname "$0")/../../.." && pwd)"
sha="$(git -C "$root" rev-parse --short HEAD)"
tgz="$(mktemp -t surya-perf-pkg.XXXXXX.tgz)"
tar czf "$tgz" -C "$root" --exclude='app/target' --exclude='**/node_modules' --exclude='app/crates/browser/winprobe/target' app docs/perf/dtry
scp -q "$tgz" dtry:E:/surya-perf-pkg.tgz
ssh dtry "New-Item -ItemType Directory -Force -Path E:\\surya-perf | Out-Null; tar -xzf E:\\surya-perf-pkg.tgz -C E:\\surya-perf; Set-Content -Path E:\\surya-perf\\SHA -Value '$sha'; Get-ChildItem -Recurse -Path E:\\surya-perf\\app -Include *.rs,Cargo.toml,*.hlsl | ForEach-Object { \$_.LastWriteTime = Get-Date }; Write-Output shipped"
echo "sha=$sha"
rm -f "$tgz"
