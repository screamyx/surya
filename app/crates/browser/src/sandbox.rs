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
//! The two are not independent, and this cost a CI run to learn. Chromium
//! picks the SUID path whenever a `chrome-sandbox` file exists beside the
//! binary, *before* it looks at whether that file is usable, and if it is
//! not it stops there rather than trying namespaces. Measured on the CI
//! runner, 2026-09-05, with the sandbox on and namespaces available:
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
//! that file and use the namespace sandbox it can actually build; the
//! sandbox stays on. That switch is the reason this module has a
//! [`switches`] side.
//!
//! The measurement for (1) is a real `fork` + `unshare(CLONE_NEWUSER)` in
//! the child, not a read of `/proc/sys`. Three separate settings can deny
//! user namespaces (`kernel.unprivileged_userns_clone`,
//! `user.max_user_namespaces`, and on Ubuntu 24.04 the AppArmor restriction
//! `kernel.apparmor_restrict_unprivileged_userns`, which is per-profile and
//! does not show up in a sysctl read at all). The syscall answers all three
//! at once. The child only calls `unshare` and `_exit`, both plain syscalls,
//! which is what makes the fork safe in this already-threaded process.

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
    match suid_helper() {
        Some(path) => {
            let shown = path.display().to_string();
            Decision::on(format!("SUID helper {shown}"), Some(path))
        }
        None => match user_namespaces_available() {
            // `--disable-setuid-sandbox` is not a weakening. Without it
            // Chromium sees the mode-0755 chrome-sandbox the cef build
            // script drops beside the binary, commits to the SUID path, and
            // aborts; with it, it builds the namespace sandbox instead.
            true => Decision::on("unprivileged user namespaces", None)
                .with_switch("disable-setuid-sandbox"),
            false => Decision::off(
                "no user namespaces (unshare(CLONE_NEWUSER) refused) and no SUID \
                 chrome-sandbox beside the binary; run the packaged install.sh, \
                 which chowns chrome-sandbox to root and sets mode 4755",
            ),
        },
    }
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
