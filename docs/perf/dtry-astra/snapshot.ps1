param([Parameter(Mandatory=$true)][ValidateSet('threaded', 'latency', 'present', 'combined', 'instrumented')][string]$Name)
$ErrorActionPreference = 'Stop'
$source = 'E:\surya-astra-target\release'
$destination = "E:\surya-astra-bin-$Name"
$stage = "E:\surya-astra-snapshot-stage-$Name"
$sha = (Get-Content E:\surya-astra\SHA).Trim()
$build = Get-Content E:\surya-astra-build.log
if ($build[0] -notmatch "sha=$sha " -or $build[-1] -notmatch '^BUILD_EXIT=0 ') {
  throw 'The build log does not confirm success for the current source SHA'
}
if (Test-Path $destination) {
  if ((Get-Content "$destination\SHA").Trim() -eq $sha) { "snapshot=$Name sha=$sha existing=1"; exit 0 }
  throw 'An existing snapshot has a different SHA; preserve it for reproducibility'
}
New-Item -ItemType Directory -Force -Path $stage | Out-Null
Get-ChildItem $source -File | Where-Object {
  $_.Extension -in @('.exe', '.dll', '.pak', '.dat', '.bin', '.json')
} | Copy-Item -Destination $stage -Force
Copy-Item "$source\locales" "$stage\locales" -Recurse -Force
Set-Content "$stage\SHA" $sha
Copy-Item E:\surya-astra-build.log "$stage\build.log"
Move-Item $stage $destination
"snapshot=$Name sha=$sha existing=0"
