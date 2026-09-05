//! The engine and the surya-mcp sidecar must resolve the mail channel to the
//! same two paths.
//!
//! When they diverged, nothing failed: the sidecar wrote one file and the
//! engine watched another, and mail simply stopped arriving with no error on
//! either side. That is the failure mode worth a test.
//!
//! Its own binary because it sets process-global environment variables, and
//! CI runs `cargo test --jobs 2`: siblings in one binary are threads in one
//! process, so a sibling reading the environment would see this test's values.

use std::path::PathBuf;

use zeron_engine::MailIngressPaths;

/// The sidecar's resolution, restated from `crates/mcp/src/config.rs`.
///
/// `surya-mcp` is a binary-only crate, so this cannot call the real function.
/// `the_sidecar_still_reads_these_names` below is the tripwire for that: it
/// fails if the sidecar stops naming these variables, which is when this
/// restatement would silently go stale.
fn sidecar_paths() -> (PathBuf, PathBuf) {
    let env_path = |k: &str| {
        std::env::var_os(k)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    let runtime_dir = || match env_path("XDG_RUNTIME_DIR") {
        Some(dir) => dir.join("surya"),
        None => std::env::temp_dir().join(format!("surya-{}", unsafe { libc::getuid() })),
    };
    let home_dir = || match env_path("HOME") {
        Some(home) => home.join(".surya"),
        None => runtime_dir(),
    };
    (
        env_path("SURYA_MAIL_SOCKET").unwrap_or_else(|| runtime_dir().join("mail.sock")),
        env_path("SURYA_MAIL_LOG").unwrap_or_else(|| home_dir().join("mail.jsonl")),
    )
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
    // SAFETY: this binary holds one test, so nothing else in the process reads
    // the environment while it is being changed.
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
    if cfg!(unix) {
        let socket = MailIngressPaths::detect().socket.expect("a socket on unix");
        let name = socket.parent().and_then(|p| p.file_name()).unwrap();
        assert!(
            name.to_string_lossy().starts_with("surya-"),
            "the fallback is uid-suffixed so two users do not share one socket: {socket:?}"
        );
    }

    unsafe {
        // 3. No HOME either: both sides put the jsonl under the runtime dir.
        std::env::remove_var("HOME");
    }
    assert_agrees("no runtime dir and no home");
}

/// The tripwire for the restatement above: if the sidecar stops reading these
/// names, `sidecar_paths` is describing a resolution that no longer exists and
/// the agreement it checks is worthless.
#[test]
fn the_sidecar_still_reads_these_names() {
    let config = concat!(env!("CARGO_MANIFEST_DIR"), "/../mcp/src/config.rs");
    let source = std::fs::read_to_string(config).expect("read the sidecar's config");
    for name in [
        "SURYA_MAIL_SOCKET",
        "SURYA_MAIL_LOG",
        "XDG_RUNTIME_DIR",
        "HOME",
    ] {
        assert!(
            source.contains(name),
            "{name} is gone from crates/mcp/src/config.rs: update sidecar_paths() in this file to match"
        );
    }
}
