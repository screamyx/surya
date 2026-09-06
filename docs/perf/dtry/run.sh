#!/usr/bin/env bash
# Drive one measurement run on dtry from this box and print its lines.
#
#   docs/perf/dtry/run.sh <name> <seconds> [KEY=VALUE ...]
#   docs/perf/dtry/run.sh scroll-pool 40 SURYA_PUMP_TIMER=pool SURYA_SELFTEST_SCROLL=15
#
# Needs: ssh dtry (PowerShell on the far side), the tree at E:\surya-perf,
# a release surya.exe from build.ps1, and the dtry GUI slot (ask surya-remote
# over agb first; one seat at a time; say GUI FREE after).
set -u
name="$1"; secs="$2"; shift 2
envfile=$(mktemp)
for kv in "$@"; do printf '%s\r\n' "$kv" >> "$envfile"; done
ssh dtry "New-Item -ItemType Directory -Force -Path E:\\surya-perf-runs | Out-Null" || exit 1
scp -q "$envfile" "dtry:E:/surya-perf-runs/$name.env" || exit 1
rm -f "$envfile"
ssh dtry "schtasks /delete /tn surya-perf-$name /f 2>\$null | Out-Null; schtasks /create /tn surya-perf-$name /tr \"conhost.exe --headless powershell.exe -NoProfile -ExecutionPolicy Bypass -File E:\\surya-perf\\docs\\perf\\dtry\\run.ps1 -Name $name -Seconds $secs\" /sc once /st 00:00 /it /f | Out-Null; schtasks /run /tn surya-perf-$name | Out-Null; Write-Output started" || exit 1
# The run ends by itself; wait for the stop marker.
for _ in $(seq 1 $(( secs / 5 + 30 ))); do
  sleep 5
  if ssh dtry "Select-String -Path E:\\surya-perf-runs\\$name.launch.log -Pattern '^stopped' -Quiet" 2>/dev/null | grep -q True; then break; fi
done
echo "== $name"
ssh dtry "Get-Content E:\\surya-perf-runs\\$name.launch.log; Select-String -Path E:\\surya-perf-runs\\$name.out.log -Pattern 'selftest:|browser: (pump|clock|frame rate|timer|create)|MAIN THREAD QUIET' | ForEach-Object { \$_.Line } | Out-String -Width 4000; schtasks /delete /tn surya-perf-$name /f | Out-Null"
