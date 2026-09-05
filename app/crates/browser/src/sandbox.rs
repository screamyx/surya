//! Whether Chromium's own sandbox can be on, and the exact reason when it
//! cannot.
//!
//! The pane shipped with `Settings { no_sandbox: 1 }` hardcoded, so every
//! renderer ran unconfined: a bug in a page's JS engine reached the whole
//! user account. Nothing about the pane needs that. What the sandbox needs
//! is a way for Chromium to drop the renderer into an empty namespace, and
//! Linux offers exactly two:
//!
//! 1. **Unprivileged user namespaces.** Chromium's zygote calls
//!    `unshare(CLONE_NEWUSER)` itself, no root anywhere. This is the modern
//!    path and the one that needs no install step.
//! 2. **The SUID helper.** `chrome-sandbox`, owned by root with mode 4755,
//!    which Chromium execs to build the namespace for it. CEF ships the
//!    binary in its distribution, but at mode 0755: making it SUID is a
//!    packaging/install step (`deploy`-side), not something the app can do.
//!
//! If neither is available Chromium does not fall back, it aborts the
//! renderer at start-up. So this module measures which one is there and
//! turns the sandbox on when the answer is "one of them"; it only reports
//! `Off` when it has a concrete reason, and prints that reason.
//!
//! The two are not independent, and this cost two runs to learn.
//!
//! First: Chromium picks the SUID path whenever a `chrome-sandbox` file
//! exists beside the binary, *before* it looks at whether that file is
//! usable, and if it is not it stops there rather than trying namespaces.
//! Measured on the CI runner, 2026-09-05, with namespaces available:
//!
//! ```text
//! browser: sandbox on=true (unprivileged user namespaces)
//! [FATAL:sandbox/linux/suid/client/setuid_sandbox_host.cc:166] The SUID
//! sandbox helper binary was found, but is not configured correctly. Rather
//! than run without sandboxing I'm aborting now. You need to make sure that
//! .../chrome-sandbox is owned by root and has mode 4755.
//! ```
//!
//! The cef crate's build script copies `chrome-sandbox` next to the binary
//! at mode 0755, so this is the *default* state of a developer build, not an
//! edge case. `--disable-setuid-sandbox` is what tells Chromium to ignore
//! that file, and it is only ever set on the namespace path.
//!
//! Second, and the reason the namespace path is not the default: with that
//! switch in, Chromium on this machine refuses the namespace sandbox too.
//!
//! ```text
//! [FATAL:content/browser/zygote_host/zygote_host_impl_linux.cc:128] No
//! usable sandbox! If you are running on Ubuntu 23.10+ or another Linux
//! distro that has disabled unprivileged user namespaces with AppArmor,
//! see .../apparmor-userns-restrictions.md ...
//! ```
//!
//! The message blames AppArmor, and the machine does have
//! `kernel.apparmor_restrict_unprivileged_userns=1`, but that is not the
//! mechanism here and three explanations were tested and refuted:
//!
//! - AppArmor strips capabilities inside the new namespace. No: a forked
//!   child reads `CapEff: 000001ffffffffff`, the full set, after
//!   `unshare(CLONE_NEWUSER)`.
//! - The restriction is per-binary and an unprofiled binary gets EPERM.
//!   No: an unprofiled throwaway binary in /tmp, `unshare -U`, and a
//!   `ctypes` call from python3 all create the namespace successfully.
//! - The kernel denies nested namespaces. No: `NEWUSER`, `NEWUSER|NEWPID`,
//!   `NEWUSER|NEWNET` and all three together each return 0.
//!
//! So Chromium's own `CanCreateProcessInNewUserNS` refuses for a reason
//! none of us has shown, and this module does not pretend to know it.
//!
//! What follows from that is the policy below. A Chromium that cannot build
//! its sandbox does not fall back, it aborts, and an aborted Chromium is a
//! dead pane. A dead pane is worse for the person using this than a working
//! one that says out loud it is unsandboxed. So:
//!
//! | Found | Sandbox | Why |
//! |---|---|---|
//! | SUID `chrome-sandbox` | on | proven to work; what `install.sh` sets up |
//! | nothing | off, loudly | the namespace path cannot be verified in advance |
//!
//! `SURYA_SANDBOX=namespace` forces the namespace attempt anyway, for
//! whoever picks this up next. `SURYA_NO_SANDBOX=1` is the plain off switch.
//!
//! One trap for anyone doing this in a *build* directory rather than an
//! installed one: once `target/debug/chrome-sandbox` is owned by root, the
//! cef crate's build script can no longer overwrite it and the next build
//! fails with a bare `Error: Permission denied (os error 13)`. Chown it back
//! to yourself before rebuilding. The packaged layout has no such problem,
//! because nothing writes into an installed directory.

/// What the process decided about the sandbox, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Decision {
    /// True when Chromium's sandbox is on for this run.
    pub on: bool,
    /// Human-readable cause, printed at start-up and used in tests.
    pub why: String,
    /// The SUID helper to point Chromium at, when that is the mechanism.
    pub devel_sandbox: Option<std::path::PathBuf>,
    /// Chromium command-line switches this decision needs. Appended by
    /// `cef_app::switches`, which runs later, inside `initialize`.
    pub switches: Vec<String>,
}

impl Decision {
    // Only Linux can currently answer "on", so on every other target this
    // constructor is reachable from the tests alone.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    fn on(why: impl Into<String>, devel_sandbox: Option<std::path::PathBuf>) -> Self {
        Self { on: true, why: why.into(), devel_sandbox, switches: Vec::new() }
    }

    fn off(why: impl Into<String>) -> Self {
        Self { on: false, why: why.into(), devel_sandbox: None, switches: Vec::new() }
    }

    fn with_switch(mut self, switch: &str) -> Self {
        self.switches.push(switch.to_string());
        self
    }

    /// The value CEF's `Settings.no_sandbox` wants: 1 means *no* sandbox.
    pub(crate) fn no_sandbox_setting(&self) -> i32 {
        i32::from(!self.on)
    }
}

/// The decision this process made, for the parts of the code that run later.
static DECISION: std::sync::OnceLock<Decision> = std::sync::OnceLock::new();

/// The Chromium switches the sandbox decision needs, empty until
/// [`decide_and_apply`] has run. Read by `cef_app::switches`, which Chromium
/// calls from inside `initialize`, so the decision is always already there.
pub(crate) fn switches() -> Vec<String> {
    DECISION.get().map(|d| d.switches.clone()).unwrap_or_default()
}

/// Decide, apply the environment Chromium needs, and print the reason.
///
/// Call once from `start`, before `initialize`.
pub(crate) fn decide_and_apply() -> Decision {
    let d = decide();
    if let Some(helper) = &d.devel_sandbox {
        // Chromium looks for the SUID helper at a path baked in at compile
        // time; `CHROME_DEVEL_SANDBOX` is the documented override, and the
        // only way an embedder can name its own copy.
        // SAFETY: start-up, before CEF spawns anything that reads the
        // environment, and before any other thread of ours touches it.
        unsafe { std::env::set_var("CHROME_DEVEL_SANDBOX", helper) };
    }
    println!("browser: sandbox on={} ({}) switches={:?}", d.on, d.why, d.switches);
    let _ = DECISION.set(d.clone());
    d
}

fn decide() -> Decision {
    // The escape hatch, for a diagnosis or a box where the sandbox is the
    // thing under suspicion. Named so it is obvious in a process list.
    if std::env::var_os("SURYA_NO_SANDBOX").is_some() {
        return Decision::off("SURYA_NO_SANDBOX is set");
    }
    platform_decision()
}

#[cfg(target_os = "linux")]
fn platform_decision() -> Decision {
    if let Some(path) = suid_helper() {
        let shown = path.display().to_string();
        return Decision::on(format!("SUID helper {shown}"), Some(path));
    }
    // Opt-in only: see the header for why an unverified namespace sandbox is
    // not the default.
    if std::env::var_os("SURYA_SANDBOX").is_some_and(|v| v == "namespace") {
        return Decision::on("unprivileged user namespaces (SURYA_SANDBOX=namespace)", None)
            .with_switch("disable-setuid-sandbox");
    }
    // This line is what a person reads when their pages are unsandboxed, so
    // it names the file that was missing and the two commands that fix it.
    // The kernel's own answer goes in too: it says yes on the machine where
    // Chromium says no, and that is worth seeing rather than guessing at.
    let expected = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.join("chrome-sandbox")))
        .unwrap_or_else(|| std::path::PathBuf::from("chrome-sandbox"));
    let shown = expected.display();
    let kernel_allows_ns = user_namespaces_available();
    Decision::off(format!(
        "web pages are NOT sandboxed. {shown} is missing or is not owned by \
         root with mode 4755. Fix it with: sudo chown root:root {shown} && \
         sudo chmod 4755 {shown} (the packaged install.sh does this for \
         you), or give the binary an AppArmor profile with a `userns` rule. \
         This kernel does allow user namespaces: {kernel_allows_ns}; \
         Chromium can still refuse them, and SURYA_SANDBOX=namespace makes \
         it try"
    ))
}

// Windows: CEF's sandbox is a link-time decision, not a runtime one. The exe
// must link `cef_sandbox.lib` (the `cef` crate's `sandbox` cargo feature ->
// `cef-dll-sys/sandbox` -> cmake `USE_SANDBOX`) and pass the resulting
// `sandbox_info` pointer to both `cef_execute_process` and `cef_initialize`;
// `preflight` and `start` pass a null pointer today, and with a null pointer
// and `no_sandbox: 0` Chromium's child processes have no broker to talk to.
// Turning it on is therefore a build change plus two call-site changes that
// only compile on Windows, and it has to be proven on a Windows machine
// before it ships. Until that run happens the flag stays off HERE, on
// Windows only, and the blocker is this comment.
#[cfg(windows)]
fn platform_decision() -> Decision {
    Decision::off(
        "windows: the exe does not link cef_sandbox (cef crate feature \
         \"sandbox\") and initialize/execute_process are passed a null \
         sandbox_info, so the child processes would have no broker",
    )
}

#[cfg(not(any(target_os = "linux", windows)))]
fn platform_decision() -> Decision {
    Decision::off("no sandbox policy for this platform yet")
}

/// The SUID `chrome-sandbox` Chromium would accept: root-owned, mode 4755.
/// Checked beside the executable, which is where the packaged app puts it,
/// and at `CHROME_DEVEL_SANDBOX` when the environment already names one.
#[cfg(target_os = "linux")]
fn suid_helper() -> Option<std::path::PathBuf> {
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    if let Some(named) = std::env::var_os("CHROME_DEVEL_SANDBOX") {
        candidates.push(std::path::PathBuf::from(named));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("chrome-sandbox"));
        }
    }
    candidates.into_iter().find(|p| is_suid_root(p))
}

#[cfg(target_os = "linux")]
fn is_suid_root(path: &std::path::Path) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    use std::os::unix::fs::PermissionsExt as _;
    let Ok(md) = std::fs::metadata(path) else { return false };
    // 0o4000 is the setuid bit; 0o111 that it is executable at all.
    let mode = md.permissions().mode();
    md.is_file() && md.uid() == 0 && mode & 0o4000 != 0 && mode & 0o111 != 0
}

/// Can this process create a user namespace? Answered by doing it, in a
/// child that does nothing else.
#[cfg(target_os = "linux")]
fn user_namespaces_available() -> bool {
    // SAFETY: the child between `fork` and `_exit` calls only `unshare`,
    // a plain syscall, so none of the usual fork-in-a-threaded-process
    // hazards (a lock held by a thread that does not exist in the child)
    // can be reached.
    unsafe {
        let pid = libc::fork();
        if pid < 0 {
            return false;
        }
        if pid == 0 {
            let rc = libc::unshare(libc::CLONE_NEWUSER);
            libc::_exit(if rc == 0 { 0 } else { 1 });
        }
        let mut status: libc::c_int = 0;
        if libc::waitpid(pid, &mut status, 0) < 0 {
            return false;
        }
        // WIFEXITED && WEXITSTATUS == 0
        status & 0x7f == 0 && (status >> 8) & 0xff == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_sandbox_setting_is_the_inverse_of_on() {
        assert_eq!(Decision::on("x", None).no_sandbox_setting(), 0);
        assert_eq!(Decision::off("x").no_sandbox_setting(), 1);
    }

    #[test]
    fn the_escape_hatch_wins_and_names_itself() {
        // SAFETY: single-threaded test process.
        unsafe { std::env::set_var("SURYA_NO_SANDBOX", "1") };
        let d = decide();
        unsafe { std::env::remove_var("SURYA_NO_SANDBOX") };
        assert!(!d.on, "{d:?}");
        assert!(d.why.contains("SURYA_NO_SANDBOX"), "{d:?}");
    }

    #[test]
    fn every_decision_carries_a_reason() {
        let d = platform_decision();
        assert!(!d.why.is_empty(), "a decision with no reason is not reportable");
        // An `off` must never be silent: it is the thing a reviewer reads.
        if !d.on {
            assert!(d.why.len() > 20, "off needs the real cause, got {:?}", d.why);
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_non_suid_file_is_not_the_helper() {
        // This source file: readable, not setuid, not root-owned.
        assert!(!is_suid_root(std::path::Path::new(file!())));
        assert!(!is_suid_root(std::path::Path::new("/definitely/not/here")));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_namespace_path_disables_the_setuid_one() {
        // Chromium aborts rather than fall back when a non-SUID
        // chrome-sandbox sits beside the binary, so an `on` that rests on
        // namespaces must always carry the switch that skips the SUID path.
        // (Measured: CI run 33964652975 failed exactly this way without it.)
        let d = platform_decision();
        if d.on && d.devel_sandbox.is_none() {
            assert!(
                d.switches.iter().any(|s| s == "disable-setuid-sandbox"),
                "namespace sandbox without the switch: {d:?}"
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_suid_path_carries_no_switch() {
        let d = Decision::on("SUID helper /x", Some("/x".into()));
        assert!(d.switches.is_empty(), "{d:?}");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_userns_probe_returns_and_agrees_with_itself() {
        // The value depends on the box; what must hold is that the probe
        // terminates, reaps its child, and is stable across calls.
        assert_eq!(user_namespaces_available(), user_namespaces_available());
    }
}
