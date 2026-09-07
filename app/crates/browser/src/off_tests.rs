//! Tests for `off.rs`.
//!
//! All pure: no CEF, no window, no gpui. What is under test is the choice of
//! words and the lock probe that picks between them.

use super::*;

/// A directory of our own under the system temp dir, removed on drop. The
/// crate has no dev-dependencies and one test needs a real filesystem entry.
struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!("surya-off-{tag}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&path).expect("temp dir");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The finding itself: the second window must say another window has the
/// browser, and must not send the person after an environment variable as
/// the explanation. Before the fix every reason drew the `ByRequest` words.
#[test]
fn a_held_cache_names_the_other_window_first() {
    let held = note(Off::CacheHeld);
    assert!(
        held.line.contains("Another surya window"),
        "the first line must name the other window, got {:?}",
        held.line
    );
    assert!(
        !held.line.contains("SURYA_"),
        "no variable name in the line a person reads first, got {:?}",
        held.line
    );
    // The way out is offered, but quietly, on the second line.
    assert_eq!(
        held.hint,
        Some("Or set SURYA_CEF_CACHE to a different folder to run both at once.")
    );
}

/// The three reasons must be distinguishable on screen, which is the whole
/// point of keeping them apart in the first place.
#[test]
fn every_reason_reads_differently() {
    let lines = [
        note(Off::ByRequest).line,
        note(Off::CacheHeld).line,
        note(Off::StartFailed).line,
    ];
    for (i, a) in lines.iter().enumerate() {
        for b in lines.iter().skip(i + 1) {
            assert_ne!(a, b, "two reasons draw the same line");
        }
    }
}

/// A start that failed with a free cache must not blame another window: CEF
/// also refuses to start when its runtime files are missing.
#[test]
fn a_failed_start_with_a_free_cache_blames_nobody() {
    let failed = note(Off::StartFailed);
    assert!(!failed.line.contains("Another"), "got {:?}", failed.line);
    assert_eq!(failed.hint, Some("Set SURYA_CEF_LOG to a file path and reopen to record why."));
}

#[test]
fn an_empty_cache_directory_is_not_held() {
    let dir = TempDir::new("free");
    assert!(!cache_is_held(dir.path()));
}

/// The Windows marker, which is the one cef3 read on winbox.
#[test]
fn a_lockfile_means_the_cache_is_held() {
    let dir = TempDir::new("lockfile");
    std::fs::write(dir.path().join("lockfile"), b"").expect("write lockfile");
    assert!(cache_is_held(dir.path()));
}

/// The POSIX marker is a symlink to a target that does not exist, so
/// `Path::exists` answers false for it and `symlink_metadata` answers true.
#[cfg(unix)]
#[test]
fn a_dangling_singleton_symlink_means_the_cache_is_held() {
    let dir = TempDir::new("singleton");
    let link = dir.path().join("SingletonLock");
    std::os::unix::fs::symlink("host-1234", &link).expect("symlink");
    assert!(!link.exists(), "the target is meant to be absent");
    assert!(cache_is_held(dir.path()));
}

/// A cache directory that exists but holds something else is free.
#[test]
fn an_unrelated_file_does_not_look_like_a_lock() {
    let dir = TempDir::new("unrelated");
    std::fs::write(dir.path().join("Cookies"), b"").expect("write");
    assert!(!cache_is_held(dir.path()));
}

/// `lib.rs` stores the reason as a `u8` so the render path reads it without
/// a lock. Zero must stay "running".
#[test]
fn the_reason_survives_the_atomic() {
    for off in [Off::ByRequest, Off::CacheHeld, Off::StartFailed] {
        assert_eq!(Off::from_code(off.code()), Some(off));
        assert_ne!(off.code(), 0, "zero is reserved for a running browser");
    }
    assert_eq!(Off::from_code(0), None);
    assert_eq!(Off::from_code(9), None);
}
