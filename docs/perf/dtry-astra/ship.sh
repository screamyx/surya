#!/usr/bin/env bash
set -eu
root=$(git rev-parse --show-toplevel)
package=/store/agent-worktrees/surya-browser-astra/ship.tgz
tar -czf "$package" -C "$root" --exclude='**/target' --exclude='**/node_modules' app docs/perf/dtry-astra
scp -q "$package" dtry:E:/surya-astra.tgz
sha=$(git rev-parse HEAD)
ssh -o BatchMode=yes dtry "New-Item -ItemType Directory -Force E:\\surya-astra | Out-Null; tar -xzf E:\\surya-astra.tgz -C E:\\surya-astra; Set-Content E:\\surya-astra\\SHA '$sha'; Get-ChildItem E:\\surya-astra\\app -Recurse -Include *.rs,Cargo.toml,*.hlsl | ForEach-Object { \$_.LastWriteTime=Get-Date }; Write-Output 'shipped $sha'"
