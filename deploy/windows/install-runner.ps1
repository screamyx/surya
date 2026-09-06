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
  pwsh -ExecutionPolicy Bypass -File deploy\windows\install-runner.ps1 -Token <token from step 1>

  # unattended, as a built-in account, which is how dtry runs it:
  pwsh -ExecutionPolicy Bypass -File deploy\windows\install-runner.ps1 `
    -Token <token> -LogonAccount "NT AUTHORITY\SYSTEM"

For a real user account it asks for the Windows password once, at the
prompt, so the service can log on as them. For a built-in account
(LocalSystem, NetworkService, LocalService) there is no password and no
prompt, so the whole thing runs unattended. Nothing is stored on disk by
this script except the runner's own configuration, which the runner
encrypts.

  ... -Root E:\actions-runner-surya   where the runner lives. Default is on
                                      E:, not C:, so neither the runner nor
                                      its _work directory eats the system
                                      drive.
  ... -LogonAccount dtry\ajim         the account the service runs as.
  ... -Priority Idle                  or BelowNormal. See the note below.
  ... -Version 2.337.0                pin the runner version.
  ... -ExpectedSha256 <hash>          fail unless the download matches.
  ... -Ifeo                           also pin the priority machine-wide
                                      through the registry. OFF by default,
                                      and read the note below before you
                                      turn it on: it applies to every
                                      GitHub Actions runner on the box, not
                                      just this one.
  ... -Uninstall                      stop, unconfigure and remove it.

WHAT KEEPS THE BUILD AT IDLE PRIORITY

The primary mechanism is not in this script. It is one line in the CI job,
`(Get-Process -Id $PID).PriorityClass = "Idle"`, which runs inside every
build step. Windows gives a child process its parent's priority class, so
that line covers cargo, rustc and link.exe. It is scoped to surya's own job
and it cannot affect anything else on the machine.

-Ifeo is the optional second belt and it is off for a reason. See the note
further down.

Needs:
  - administrator
  - PowerShell 7 (`pwsh`) on PATH. The CI job's steps all run `shell: pwsh`.
    Windows PowerShell 5.1 is not it. This script checks.
  - the same tools build.ps1 needs: Rust with the MSVC toolchain, Visual
    Studio Build Tools with the C++ workload, CMake, Ninja, and CEF already
    extracted at the CEF_PATH the workflow passes.
This script installs none of those. It does not run cargo.
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
    [switch]$Ifeo,
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

# Every step of the CI job declares `shell: pwsh`. If PowerShell 7 is not on
# PATH the runner registers, takes the job, and then fails every single step
# with "pwsh: command not found" after the checkout has already run. That is
# what happened on the first live install on dtry, so the check is here
# rather than in a comment.
# No ?. or any other PowerShell 7 syntax in this check, and none above it.
# The whole point is that this script may be started from Windows PowerShell
# 5.1 on a box that has no pwsh yet, and a 7-only operator anywhere in the
# file makes 5.1 fail to PARSE it, so the reader gets a syntax error instead
# of the message below.
$pwshCmd = Get-Command pwsh -ErrorAction SilentlyContinue
if (-not $pwshCmd) {
    throw @'
PowerShell 7 (pwsh) is not on PATH, and every step of the CI job runs with
"shell: pwsh". Windows PowerShell 5.1, which you are probably in now, is a
different thing. Install it and run this again:

  winget install --id Microsoft.PowerShell --source winget

Then open a new administrator PowerShell so PATH is refreshed.
'@
}
Write-Host "== pwsh at $($pwshCmd.Source)"

# ---------------------------------------------------------------------------
# -Ifeo: the optional registry belt, and why it is off by default
#
# The Windows service manager has no priority setting. A service's priority
# is the priority of the process it starts, so a machine-wide setting has to
# attach to the image, through Image File Execution Options:
#
#   HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\
#     Runner.Listener.exe\PerfOptions   CpuPriorityClass = 1   (Idle)
#     Runner.Worker.exe\PerfOptions     CpuPriorityClass = 1
#
# THAT KEY IS PER IMAGE NAME, NOT PER SERVICE. Every GitHub Actions runner on
# the machine ships executables with these exact names. dtry already runs
# haktui's runner out of C:\gha-runner, and writing this key would drop that
# one to Idle too, on its next restart, silently. Nobody asked for that. So
# -Ifeo is opt-in, and the primary mechanism is the line inside surya's own
# CI job, which cannot touch anything else:
#
#   (Get-Process -Id $PID).PriorityClass = "Idle"
#
# Windows gives a child its parent's priority class, so that one line carries
# down to cargo, rustc and link.exe for surya's build and no one else's.
#
# Both images are set when -Ifeo is on, because the listener starts the
# worker and the worker starts the job's shell.
#
# The numbers below are a registry contract, not something this script can
# read back from an API. So it does not trust them: with -Ifeo the script
# restarts the service, finds THIS runner's listener by its path, reads the
# live priority and throws if it is not what was asked for. Finding it by
# path matters: `Get-Process -Name Runner.Listener` on dtry returns haktui's
# listener first, at Normal, and the check would fail on the wrong process.
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
    # Only the one value this script created. The Image File Execution
    # Options key for an executable is shared: a debugger, a mitigation
    # policy or another tool's settings can live under the same key, and
    # deleting it whole would take those with it. If PerfOptions is empty
    # afterwards it was ours alone, so that subkey goes too, and the image
    # key itself if nothing else is left under it.
    foreach ($image in $images) {
        $imageKey = Join-Path $ifeo $image
        $perf = Join-Path $imageKey "PerfOptions"
        if (-not (Test-Path $perf)) { Write-Host "  no PerfOptions for $image, nothing to undo"; continue }
        if ($null -ne (Get-ItemProperty -Path $perf -Name "CpuPriorityClass" -ErrorAction SilentlyContinue)) {
            Remove-ItemProperty -Path $perf -Name "CpuPriorityClass"
            Write-Host "  removed CpuPriorityClass for $image"
        } else {
            Write-Host "  $image has no CpuPriorityClass, nothing to undo"
        }
        $perfKey = Get-Item $perf
        if ($perfKey.ValueCount -eq 0 -and $perfKey.SubKeyCount -eq 0) {
            Remove-Item -Path $perf
            $imgKey = Get-Item $imageKey
            if ($imgKey.ValueCount -eq 0 -and $imgKey.SubKeyCount -eq 0) { Remove-Item -Path $imageKey }
        }
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
    if ($Ifeo) {
        # Only with -Ifeo, and only the one value. Another runner on this box
        # may have set CpuPriorityClass for its own reasons; removing a key
        # this run did not write is not this script's business.
        Remove-ImagePriority
    } else {
        Write-Host "  no -Ifeo, leaving the registry alone (pass -Ifeo to undo an -Ifeo install)"
    }
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

# A built-in service account has no password and config.cmd must not be given
# one: --windowslogonpassword with an empty value fails, and prompting for a
# password that does not exist is what stopped the first unattended install on
# dtry. Anything else is a real account and does need one.
# These accounts are spelled a dozen ways: with or without the "NT
# AUTHORITY\" domain, with or without the space in "Network Service", and
# "SYSTEM" on its own. Listing every spelling is how one gets missed, so
# normalise instead: drop the domain, drop the spaces, uppercase, compare.
function Test-BuiltInAccount([string]$Account) {
    $bare = $Account.Trim()
    if ($bare -match '^\s*NT AUTHORITY\\(.+)$') { $bare = $Matches[1] }
    $bare = ($bare -replace '\s', '').ToUpperInvariant()
    return $bare -in @("SYSTEM", "LOCALSYSTEM", "NETWORKSERVICE", "LOCALSERVICE")
}
$isBuiltIn = Test-BuiltInAccount $LogonAccount

$configArgs = @(
    "--unattended", "--replace",
    "--url", $Url,
    "--token", $Token,
    "--name", $Name,
    "--labels", $Labels,
    "--work", "_work",
    "--runasservice",
    "--windowslogonaccount", $LogonAccount
)

Write-Host ""
Write-Host "The service will log on as $LogonAccount."
if ($isBuiltIn) {
    Write-Host "That is a built-in account: no password, nothing to prompt for."
    Write-Host "== configuring, label $Labels"
    Push-Location $Root
    try {
        .\config.cmd @configArgs
        if ($LASTEXITCODE -ne 0) { throw "config.cmd failed with exit $LASTEXITCODE" }
    } finally { Pop-Location }
} else {
    # Read the password at the prompt into a SecureString, hand it to
    # config.cmd, and drop it. It is never written to disk, never put in an
    # environment variable, and never printed.
    $secure = Read-Host -Prompt "Windows password for $LogonAccount" -AsSecureString
    $bstr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure)
    try {
        $plain = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($bstr)
        Write-Host "== configuring, label $Labels"
        Push-Location $Root
        try {
            .\config.cmd @configArgs --windowslogonpassword $plain
            if ($LASTEXITCODE -ne 0) { throw "config.cmd failed with exit $LASTEXITCODE" }
        } finally { Pop-Location }
    } finally {
        [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($bstr)
        Remove-Variable plain -ErrorAction SilentlyContinue
    }
}

$svc = Get-RunnerService
if (-not $svc) { throw "config.cmd did not leave an actions.runner.* service behind" }

if ($Ifeo) {
    Write-Host "== -Ifeo: setting CpuPriorityClass on the runner images, MACHINE WIDE"
    Write-Host "   every Actions runner on this box is affected, not just $Name"
    Set-ImagePriority

    # config.cmd started the service before the registry entries existed, so
    # the listener is running at normal priority right now. Restart it so the
    # kernel reads the key, then check what actually happened.
    Write-Host "== restarting $($svc.Name) so the priority takes effect"
    Restart-Service $svc.Name -Force
    Start-Sleep -Seconds 5

    $rootPrefix = [IO.Path]::GetFullPath($Root).TrimEnd('\') + '\'
    # By path, not by name. dtry runs haktui's runner out of C:\gha-runner
    # with an executable of exactly this name, and -Name returns whichever
    # started first. Get-Process needs elevation to read Path for a process
    # owned by another account, which this script has.
    $listener = Get-Process -Name "Runner.Listener" -ErrorAction SilentlyContinue |
        Where-Object {
            $path = $null
            try { $path = $_.Path } catch { }
            # The trailing separator matters. Without it, -Root
            # E:\actions-runner-surya also matches a listener living in
            # E:\actions-runner-surya-other, and the check would read the
            # priority of the wrong runner.
            $path -and $path.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)
        } | Select-Object -First 1
    if (-not $listener) {
        throw "no Runner.Listener running out of $Root after the restart. Other runners on this box are not this one; check `Get-Service $($svc.Name)`."
    }
    $actual = $listener.PriorityClass
    Write-Host "== this runner's listener (pid $($listener.Id), $($listener.Path)) is at $actual"
    if ("$actual" -ne $Priority) {
        throw "asked for $Priority, got $actual. The CpuPriorityClass value $priorityValue is wrong for $Priority on this Windows build; fix the map at the top of this script before trusting -Ifeo."
    }
} else {
    Write-Host "== no -Ifeo, so nothing machine-wide was changed."
    Write-Host "   The build runs at $Priority because ci.yml sets it inside the job."
}

Write-Host ""
Write-Host "== done"
Write-Host "  service: $($svc.Name), logs on as $LogonAccount"
Write-Host "  runner:  $Name, labels $Labels, at $Root"
Write-Host "  ci.yml windows job targets [self-hosted, $Labels]"
Write-Host "  priority: set per step by ci.yml$(if ($Ifeo) { ", and machine-wide by -Ifeo" })"
Write-Host ""
Write-Host "  check it registered:  gh api /repos/screamyx/surya/actions/runners"
Write-Host "  remove it again:      pwsh -File deploy\windows\install-runner.ps1 -Uninstall -Token <remove token>"
