<#
Install the GitHub Actions runner on dtry as a Windows service, labelled
`surya-win`, running at idle CPU priority.

This is the runner .github/workflows/ci.yml's `windows` job targets. It
replaces the two Linux runners on pc-ajim for the build the owner actually
uses (decision 28: Windows is the product, Linux is a test bench).

  # 1. get a registration token. It is valid for one hour. It is never
  #    written to a file in this repository.
  gh api -X POST /repos/screamyx/surya/actions/runners/registration-token --jq .token

  # 2. run this, from an ADMINISTRATOR PowerShell on dtry:
  powershell -ExecutionPolicy Bypass -File deploy\windows\install-runner.ps1 -Token <token from step 1>

It will ask for the owner's Windows password once, at the prompt, so the
service logs on as him. Nothing is stored on disk by this script except the
runner's own configuration, which the runner encrypts.

  ... -Root E:\actions-runner-surya   where the runner lives. Default is on
                                      E:, not C:, so neither the runner nor
                                      its _work directory eats the system
                                      drive.
  ... -LogonAccount dtry\ajim         the account the service runs as.
  ... -Priority Idle                  or BelowNormal. See the note below.
  ... -Version 2.337.0                pin the runner version.
  ... -ExpectedSha256 <hash>          fail unless the download matches.
  ... -Uninstall                      stop, unconfigure and remove it.

Needs: administrator, and the same tools build.ps1 needs (Rust with the MSVC
toolchain, Visual Studio Build Tools with the C++ workload, CMake, Ninja).
This script does not install those. It does not run cargo.
#>
param(
    [string]$Token = "",
    [string]$Root = "E:\actions-runner-surya",
    [string]$Url = "https://github.com/screamyx/surya",
    [string]$Name = "dtry-surya-win",
    [string]$Labels = "surya-win",
    [string]$LogonAccount = "",
    [ValidateSet("Idle", "BelowNormal")]
    [string]$Priority = "Idle",
    [string]$Version = "",
    [string]$ExpectedSha256 = "",
    [switch]$Uninstall
)
$ErrorActionPreference = "Stop"

# The registration token is a secret with a one-hour life. Refuse to take it
# from a file path by mistake, and never echo it.
if ($Token -and (Test-Path -LiteralPath $Token -ErrorAction SilentlyContinue)) {
    throw "-Token takes the token itself, not a path to a file holding it"
}

$admin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()
         ).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) { throw "run this from an administrator PowerShell: it installs a service" }

# ---------------------------------------------------------------------------
# Why the priority is set through the registry and not with sc.exe
#
# The Windows service manager has no priority setting. A service's priority
# is the priority of the process it starts, so the setting has to attach to
# the image. The documented way to do that is the Image File Execution
# Options key, PerfOptions\CpuPriorityClass, which the kernel reads when it
# creates a process from that image:
#
#   HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\
#     Runner.Listener.exe\PerfOptions   CpuPriorityClass = 1   (Idle)
#     Runner.Worker.exe\PerfOptions     CpuPriorityClass = 1
#
# Both images, because the listener starts the worker and the worker starts
# the job's shell. A process with no entry of its own inherits its parent's
# class, so setting the two runner executables carries down to cargo, rustc
# and link.exe.
#
# The numbers below are a registry contract, not something this script can
# read back from an API. So it does not trust them: after the service is
# running, the script reads the live process priority with Get-Process and
# throws if it is not what was asked for. If the mapping is ever wrong, the
# install fails on the box with the value it actually got, rather than the
# owner's machine quietly building at normal priority.
#
# ci.yml sets the same priority again inside the job
# ((Get-Process -Id $PID).PriorityClass = "Idle"). That is deliberate
# duplication: the workflow's line survives a service reinstall that forgets
# this one, and this one covers the runner's own housekeeping between jobs.
# ---------------------------------------------------------------------------
$priorityValue = @{ "Idle" = 1; "BelowNormal" = 5 }[$Priority]
$ifeo = "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options"
$images = @("Runner.Listener.exe", "Runner.Worker.exe")

function Set-ImagePriority {
    foreach ($image in $images) {
        $key = Join-Path (Join-Path $ifeo $image) "PerfOptions"
        New-Item -Path $key -Force | Out-Null
        New-ItemProperty -Path $key -Name "CpuPriorityClass" -Value $priorityValue `
            -PropertyType DWord -Force | Out-Null
        Write-Host "  $image -> CpuPriorityClass $priorityValue ($Priority)"
    }
}

function Remove-ImagePriority {
    foreach ($image in $images) {
        $key = Join-Path $ifeo $image
        if (Test-Path $key) { Remove-Item -Path $key -Recurse -Force; Write-Host "  removed IFEO for $image" }
    }
}

function Get-RunnerService {
    Get-Service | Where-Object { $_.Name -like "actions.runner.screamyx-surya.*" } | Select-Object -First 1
}

# --------------------------------------------------------------- uninstall --
if ($Uninstall) {
    Write-Host "== uninstalling"
    if (-not (Test-Path (Join-Path $Root "config.cmd"))) { throw "no runner at $Root" }
    Push-Location $Root
    try {
        $svc = Get-RunnerService
        if ($svc) { Write-Host "  stopping $($svc.Name)"; Stop-Service $svc.Name -Force }
        if (-not $Token) {
            throw "unconfiguring needs a REMOVE token: gh api -X POST /repos/screamyx/surya/actions/runners/remove-token --jq .token"
        }
        .\config.cmd remove --token $Token
        if ($LASTEXITCODE -ne 0) { throw "config.cmd remove failed with exit $LASTEXITCODE" }
    } finally { Pop-Location }
    Remove-ImagePriority
    Write-Host "== done. $Root is still on disk; delete it by hand when you are sure."
    exit 0
}

# ---------------------------------------------------------------- install --
if (-not $Token) {
    throw @"
-Token is required. Get one (valid one hour, never save it to a file):

  gh api -X POST /repos/screamyx/surya/actions/runners/registration-token --jq .token
"@
}

if (-not $LogonAccount) {
    $LogonAccount = "$env:USERDOMAIN\$env:USERNAME"
    Write-Host "no -LogonAccount given, using the account running this script: $LogonAccount"
}

$drive = (Split-Path -Qualifier $Root)
$free = [math]::Round((Get-PSDrive $drive.TrimEnd(":")).Free / 1GB, 1)
Write-Host "== $drive has $free GB free"
# The runner itself is about 300 MB. The build is what fills a disk, and that
# lives in CARGO_TARGET_DIR (E:\surya-remote-target), which ci.yml bounds
# with a weekly cargo clean above 25 GB. 5 GB here is only the install.
if ($free -lt 5) { throw "$drive has $free GB free; the runner install needs about 5 GB of headroom" }

if (-not $Version) {
    Write-Host "== looking up the latest runner release"
    $latest = Invoke-RestMethod -Uri "https://api.github.com/repos/actions/runner/releases/latest" `
        -Headers @{ "User-Agent" = "surya-install-runner" }
    $Version = $latest.tag_name.TrimStart("v")
}
Write-Host "== runner $Version"

New-Item -ItemType Directory -Force -Path $Root | Out-Null
if (Test-Path (Join-Path $Root "config.cmd")) {
    throw "a runner is already unpacked at $Root; run with -Uninstall first, or pass a different -Root"
}

$zip = Join-Path $env:TEMP "actions-runner-win-x64-$Version.zip"
$url = "https://github.com/actions/runner/releases/download/v$Version/actions-runner-win-x64-$Version.zip"
Write-Host "== downloading $url"
Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing
$hash = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
Write-Host "  sha256 $hash"
if ($ExpectedSha256) {
    if ($hash -ne $ExpectedSha256.ToLower()) {
        Remove-Item $zip -Force
        throw "sha256 mismatch: got $hash, expected $ExpectedSha256"
    }
    Write-Host "  matches -ExpectedSha256"
} else {
    Write-Host "  no -ExpectedSha256 given; compare that hash with the one on"
    Write-Host "  https://github.com/actions/runner/releases/tag/v$Version before you trust this runner"
}

Write-Host "== unpacking into $Root"
Expand-Archive -Path $zip -DestinationPath $Root -Force
Remove-Item $zip -Force

# The service needs the owner's password to log on as him. Read it at the
# prompt into a SecureString, hand it to config.cmd, and drop it. It is never
# written to disk, never put in an environment variable, and never printed.
Write-Host ""
Write-Host "The service will log on as $LogonAccount."
$secure = Read-Host -Prompt "Windows password for $LogonAccount" -AsSecureString
$bstr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure)
try {
    $plain = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($bstr)

    Write-Host "== configuring, label $Labels"
    Push-Location $Root
    try {
        .\config.cmd --unattended --replace `
            --url $Url `
            --token $Token `
            --name $Name `
            --labels $Labels `
            --work "_work" `
            --runasservice `
            --windowslogonaccount $LogonAccount `
            --windowslogonpassword $plain
        if ($LASTEXITCODE -ne 0) { throw "config.cmd failed with exit $LASTEXITCODE" }
    } finally { Pop-Location }
} finally {
    [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($bstr)
    Remove-Variable plain -ErrorAction SilentlyContinue
}

Write-Host "== setting the CPU priority of the runner images"
Set-ImagePriority

# config.cmd started the service before the registry entries existed, so the
# listener is running at normal priority right now. Restart it so the kernel
# reads the key, then check what actually happened.
$svc = Get-RunnerService
if (-not $svc) { throw "config.cmd did not leave an actions.runner.* service behind" }
Write-Host "== restarting $($svc.Name) so the priority takes effect"
Restart-Service $svc.Name -Force
Start-Sleep -Seconds 5

$listener = Get-Process -Name "Runner.Listener" -ErrorAction SilentlyContinue
if (-not $listener) { throw "$($svc.Name) is not running a Runner.Listener process after the restart" }
$actual = $listener.PriorityClass
Write-Host "== Runner.Listener priority is $actual"
if ("$actual" -ne $Priority) {
    throw "asked for $Priority, got $actual. The CpuPriorityClass value $priorityValue is wrong for $Priority on this Windows build; fix the map at the top of this script before trusting the runner."
}

Write-Host ""
Write-Host "== done"
Write-Host "  service: $($svc.Name), logs on as $LogonAccount, priority $actual"
Write-Host "  runner:  $Name, labels $Labels, at $Root"
Write-Host "  ci.yml windows job targets [self-hosted, $Labels]"
Write-Host ""
Write-Host "  check it registered:  gh api /repos/screamyx/surya/actions/runners"
Write-Host "  remove it again:      powershell -ExecutionPolicy Bypass -File deploy\windows\install-runner.ps1 -Uninstall -Token <remove token>"
