#!/usr/bin/env bash
# Announce the GUI slot over agb before calling; this script does not acquire it.
set -eu
name=$1
seconds=$2
shift 2
[[ $name =~ ^[a-z0-9-]+$ && $seconds =~ ^[0-9]+$ ]] || exit 2
envfile=$(mktemp)
trap 'rm "$envfile"' EXIT
for value in "$@"; do printf '%s\r\n' "$value" >> "$envfile"; done
ssh -o BatchMode=yes dtry 'New-Item -ItemType Directory -Force E:\surya-astra-runs | Out-Null'
scp -q "$envfile" "dtry:E:/surya-astra-runs/$name.env"
ssh -o BatchMode=yes dtry "schtasks /create /tn surya-astra-$name /tr \"conhost.exe --headless powershell.exe -NoProfile -ExecutionPolicy Bypass -File E:\\surya-astra\\docs\\perf\\dtry-astra\\run.ps1 -Name $name -Seconds $seconds\" /sc once /st 00:00 /it /f | Out-Null; schtasks /run /tn surya-astra-$name | Out-Null; Write-Output started"
