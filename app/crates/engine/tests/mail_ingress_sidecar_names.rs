//! Tripwire for `mail_ingress_paths.rs`.
//!
//! That test restates the sidecar's path resolution by hand, because
//! `surya-mcp` is a binary-only crate whose `Config` cannot be imported. A
//! restatement goes stale silently. This reads the sidecar's source and fails
//! when the variables it names stop appearing there.
//!
//! Its own binary rather than a sibling of that test: `mail_ingress_paths.rs`
//! mutates process-global environment variables, and libtest runs the tests in
//! one binary as threads in one process.

/// If the sidecar stops reading these names, `sidecar_paths()` in
/// `mail_ingress_paths.rs` describes a resolution that no longer exists, and
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
            "{name} is gone from crates/mcp/src/config.rs: update sidecar_paths() in mail_ingress_paths.rs to match"
        );
    }
}
