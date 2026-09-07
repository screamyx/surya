//! Why the pane has no page in it, and what it tells the person about that.
//!
//! Three different causes end in the same empty pane, and nothing on screen
//! tells them apart: the browser was asked to stay out of the process, another
//! surya window owns the cache directory, or CEF refused to start for some
//! other reason. Before this module they all drew one note naming an
//! environment variable most people have never set.
//!
//! The one that matters is the second. CEF holds a process singleton lock on
//! `root_cache_path` (`cef_types.h` 311-313: "Multiple application instances
//! writing to the same root_cache_path directory could result in data
//! corruption. A process singleton lock based on the root_cache_path value is
//! therefore used to protect against this."), and surya's cache path is one
//! fixed folder per user, so a second window collides by default. cef3 saw it
//! on dtry on 2026-09-06 at 21:05: the owner's own running app held the
//! default cache and the second window said only "Browser off".

use std::path::Path;

/// Why there is no browser in this process. `None` where this is used means
/// the browser is running.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Off {
    /// `SURYA_NO_BROWSER` is set. The person asked for this.
    ByRequest,
    /// CEF refused to start and the cache directory carries a singleton
    /// lock, so another CEF application owns it.
    CacheHeld,
    /// CEF refused to start and the cache is free: a missing runtime file, a
    /// sandbox refusal, a broken profile.
    StartFailed,
}

impl Off {
    /// A code for the atomic in `lib.rs`, so the hot render path reads the
    /// reason without taking a lock. Zero is reserved for "running".
    pub(crate) fn code(self) -> u8 {
        match self {
            Off::ByRequest => 1,
            Off::CacheHeld => 2,
            Off::StartFailed => 3,
        }
    }

    pub(crate) fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Off::ByRequest),
            2 => Some(Off::CacheHeld),
            3 => Some(Off::StartFailed),
            _ => None,
        }
    }
}

/// Chromium's process singleton markers inside `root_cache_path`. `lockfile`
/// is the Windows one, which is the name cef3 read on dtry; `SingletonLock`
/// is the POSIX one. Both strings are in libcef 151.3.24.
const LOCK_NAMES: [&str; 2] = ["lockfile", "SingletonLock"];

/// Whether `cache` carries a singleton lock.
///
/// Only ask this after CEF has already refused to start. A lock in a cache
/// the app is using is the normal state, so the lock alone says nothing; it
/// is the pair, a refused start and a lock, that means another window owns
/// the browser.
///
/// `symlink_metadata` and not `exists`: the POSIX marker is a symlink whose
/// target never exists, and `exists` follows it and answers false.
pub(crate) fn cache_is_held(cache: &Path) -> bool {
    LOCK_NAMES.iter().any(|name| std::fs::symlink_metadata(cache.join(name)).is_ok())
}

/// The words the pane shows in place of a page.
///
/// The words only. How they are painted belongs to whoever draws them: this
/// crate depends on gpui alone and cannot see comet's tokens, and a note
/// painted without them lands on the white page surface and disappears
/// (measured at about 1.2:1 on Windows dark).
pub struct OffNote {
    pub label: &'static str,
    /// The sentence a person reads first. Plain words, no variable names.
    pub line: &'static str,
    /// The quieter second line: the way out, where there is one.
    pub hint: Option<&'static str>,
}

/// The words for one reason.
pub(crate) fn note(off: Off) -> OffNote {
    match off {
        Off::ByRequest => OffNote {
            label: "Browser off",
            line: "SURYA_NO_BROWSER is set, so this window left Chromium out.",
            hint: None,
        },
        Off::CacheHeld => OffNote {
            label: "Browser off",
            line: "Another surya window is already using the browser. Close that window, then reopen this one.",
            hint: Some("Or set SURYA_CEF_CACHE to a different folder to run both at once."),
        },
        Off::StartFailed => OffNote {
            label: "Browser off",
            line: "The browser could not start.",
            hint: Some("Set SURYA_CEF_LOG to a file path and reopen to record why."),
        },
    }
}

#[cfg(test)]
#[path = "off_tests.rs"]
mod tests;
