//! The engine and the surya-mcp sidecar must resolve the mail channel to the
//! same two paths.
//!
//! When they diverged, nothing failed: the sidecar wrote one file and the
//! engine watched another, and mail simply stopped arriving with no error on
//! either side. That is the failure mode worth a test.
//!
//! One test in its own binary, because it sets process-global environment
//! variables. libtest runs the tests in a binary as threads in one process, so
//! a sibling reading the environment would see this test's values, and CI runs
//! `cargo test --jobs 2`. The tripwire that would otherwise sit here lives in
//! `mail_ingress_sidecar_names.rs`.

use std::path::PathBuf;

use surya_engine::MailIngressPaths;

/// The sidecar's resolution, restated from `crates/mcp/src/config.rs`.
///
/// `surya-mcp` is a binary-only crate, so this cannot call the real function.
/// `mail_ingress_sidecar_names.rs` is the tripwire for that: it fails if the
/// sidecar stops naming these variables, which is when this restatement would
/// silently go stale.
fn sidecar_paths() -> (PathBuf, PathBuf) {
    let env_path = |k: &str| {
        std::env::var_os(k)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    let runtime_dir = || match env_path("XDG_RUNTIME_DIR") {
        Some(dir) => dir.join("surya"),
        None => std::env::temp_dir().join(format!("surya-{}", current_uid())),
    };
    let home_dir = || match env_path("HOME") {
        Some(home) => home.join(".surya"),
        None => runtime_dir(),
    };
    // Both defaults hang off the engine's data directory now, so one engine's
    // mail channel moves when its data directory does (surya#216).
    let data_dir = || env_path("SURYA_DATA_DIR").unwrap_or_else(home_dir);
    (
        env_path("SURYA_MAIL_SOCKET").unwrap_or_else(|| data_dir().join("mail.sock")),
        env_path("SURYA_MAIL_LOG").unwrap_or_else(|| data_dir().join("mail.jsonl")),
    )
}

/// `getuid(2)` off unix does not exist. The sidecar and `ingress.rs` both
/// suffix the temp dir with `$USERNAME` there, so this restatement does too.
#[cfg(unix)]
fn current_uid() -> String {
    // SAFETY: getuid(2) reads a process attribute and cannot fail.
    unsafe { libc::getuid() }.to_string()
}

#[cfg(not(unix))]
fn current_uid() -> String {
    std::env::var("USERNAME").unwrap_or_else(|_| "user".into())
}

fn assert_agrees(what: &str) {
    let (socket, log) = sidecar_paths();
    let paths = MailIngressPaths::detect();
    assert_eq!(
        paths.jsonl.as_deref(),
        Some(log.as_path()),
        "{what}: the engine and the sidecar disagree about the jsonl channel"
    );
    if cfg!(unix) {
        assert_eq!(
            paths.socket.as_deref(),
            Some(socket.as_path()),
            "{what}: the engine and the sidecar disagree about the socket"
        );
    } else {
        assert!(paths.socket.is_none(), "{what}: no unix socket off unix");
    }
}

#[test]
fn both_sides_land_on_the_same_paths() {
    // SAFETY: this binary holds this one test and no other thread, so nothing
    // else in the process reads the environment while it is being changed.
    unsafe {
        // 1. An explicit override, the case that was broken: the sidecar
        //    honoured these and the engine did not, so an operator who set
        //    either one split the channel in two.
        std::env::set_var("SURYA_MAIL_SOCKET", "/tmp/smoke-override/mail.sock");
        std::env::set_var("SURYA_MAIL_LOG", "/tmp/smoke-override/mail.jsonl");
    }
    assert_agrees("explicit override");
    assert_eq!(
        MailIngressPaths::detect().jsonl,
        Some(PathBuf::from("/tmp/smoke-override/mail.jsonl")),
        "the override is used verbatim, not merely agreed upon"
    );

    unsafe {
        std::env::remove_var("SURYA_MAIL_SOCKET");
        std::env::remove_var("SURYA_MAIL_LOG");
        std::env::set_var("XDG_RUNTIME_DIR", "/tmp/smoke-runtime");
        std::env::set_var("HOME", "/tmp/smoke-home");
    }
    assert_agrees("runtime dir and home");

    unsafe {
        // 2. No runtime dir: a service launch, or a box without systemd. The
        //    sidecar falls back to a uid-suffixed temp dir; the engine used to
        //    have no socket at all here.
        std::env::remove_var("XDG_RUNTIME_DIR");
    }
    assert_agrees("no runtime dir");

    unsafe {
        // 3. No HOME either: both sides put the channel under the runtime dir.
        std::env::remove_var("HOME");
    }
    assert_agrees("no runtime dir and no home");
    if cfg!(unix) {
        let socket = MailIngressPaths::detect().socket.expect("a socket on unix");
        let name = socket.parent().and_then(|p| p.file_name()).unwrap();
        assert!(
            name.to_string_lossy().starts_with("surya-"),
            "the last-resort dir is uid-suffixed so two users do not share one socket: {socket:?}"
        );
    }

    // 4. The defect this file's test exists for since surya#216: an engine
    //    given its own data directory must get its own mail channel, both
    //    halves of it. The socket used to hang off `$XDG_RUNTIME_DIR` and the
    //    jsonl off `$HOME`, neither of which moves with `SURYA_DATA_DIR`, so a
    //    second engine shared both. On Windows there is no socket, which made
    //    the shared jsonl the only route and every message common to both.

    unsafe {
        std::env::remove_var("SURYA_MAIL_SOCKET");
        std::env::remove_var("SURYA_MAIL_LOG");
        std::env::set_var("HOME", "/var/surya-test/mail-paths-home");
        std::env::set_var("XDG_RUNTIME_DIR", "/var/surya-test/mail-paths-runtime");
        std::env::set_var("SURYA_DATA_DIR", "/var/surya-test/mail-paths-engine-a");
    }
    let a = MailIngressPaths::detect();
    assert_eq!(
        a.jsonl,
        Some(PathBuf::from("/var/surya-test/mail-paths-engine-a/mail.jsonl")),
        "the jsonl default follows the data dir, not HOME"
    );
    if cfg!(unix) {
        assert_eq!(
            a.socket,
            Some(PathBuf::from("/var/surya-test/mail-paths-engine-a/mail.sock")),
            "the socket default follows the data dir, not XDG_RUNTIME_DIR"
        );
    }
    assert_agrees("a private data dir");

    unsafe { std::env::set_var("SURYA_DATA_DIR", "/var/surya-test/mail-paths-engine-b") }
    let b = MailIngressPaths::detect();
    assert_ne!(a.jsonl, b.jsonl, "two data dirs, two jsonl files");
    assert_ne!(a.socket, b.socket, "two data dirs, two sockets");
    assert_agrees("a second private data dir");

    // An explicit override still wins over the data dir, on both sides.
    unsafe { std::env::set_var("SURYA_MAIL_LOG", "/var/surya-test/mail-paths-override.jsonl") }
    assert_eq!(
        MailIngressPaths::detect().jsonl,
        Some(PathBuf::from("/var/surya-test/mail-paths-override.jsonl")),
        "SURYA_MAIL_LOG beats SURYA_DATA_DIR"
    );
    assert_agrees("an override beside a data dir");

    unsafe {
        std::env::remove_var("SURYA_MAIL_LOG");
        std::env::remove_var("SURYA_DATA_DIR");
    }
}
