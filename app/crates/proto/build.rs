//! Stamp the crate with the checkout it was built from, so an engine and a
//! client can tell each other which commit they are. Reads git when the
//! build runs inside a checkout; a tarball build passes `ZERON_BUILD_SHA`
//! and `ZERON_BUILD_COMMIT_TIME` instead (deploy/windows/build.ps1 does).
use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (!s.is_empty()).then_some(s)
}

fn env_or(name: &str, fallback: impl FnOnce() -> Option<String>, default: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(fallback)
        .unwrap_or_else(|| default.to_string())
}

fn main() {
    println!("cargo:rerun-if-env-changed=ZERON_BUILD_SHA");
    println!("cargo:rerun-if-env-changed=ZERON_BUILD_COMMIT_TIME");
    // HEAD lives in the worktree's git dir, the branch refs in the common dir
    // (a linked worktree has no refs/heads of its own). A missing path would
    // make cargo rerun this script on every build, so only existing ones count.
    for (args, tail) in [
        (["rev-parse", "--git-dir"], "HEAD"),
        (["rev-parse", "--git-common-dir"], "refs/heads"),
    ] {
        if let Some(dir) = git(&args) {
            let path = format!("{dir}/{tail}");
            if std::path::Path::new(&path).exists() {
                println!("cargo:rerun-if-changed={path}");
            }
        }
    }
    let sha = env_or(
        "ZERON_BUILD_SHA",
        || git(&["rev-parse", "--short=9", "HEAD"]),
        "unknown",
    );
    let time = env_or(
        "ZERON_BUILD_COMMIT_TIME",
        || git(&["show", "-s", "--format=%ct", "HEAD"]),
        "0",
    );
    println!("cargo:rustc-env=ZERON_BUILD_SHA={sha}");
    println!("cargo:rustc-env=ZERON_BUILD_COMMIT_TIME={time}");
}
