//! CEF's paint-rate cap, from the display it is shown on.
//!
//! `windowless_frame_rate` had been 60 since haktui's first spike; the
//! owner's screen is 120 Hz and native Chrome paints at that. haktui
//! measured 2026-08-28: CEF capped at 60 gave 76 app frames on the scroll
//! test, at 120 it gave 122, and a still page paints nothing whatever the
//! cap, so idle cost did not move. Default is the primary display's refresh
//! rate, capped at 120; `SURYA_CEF_FPS=<n>` overrides.

use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::OnceLock;

const FALLBACK: i32 = 60;
const CAP: i32 = 120;

/// What `SURYA_CEF_FPS` said, once parsed.
#[derive(Debug, PartialEq)]
enum Override {
    /// Not set: the display decides.
    Unset,
    /// Set to something that is not a whole number: ignored, and said so.
    Bad(String),
    /// A number; `used` is `asked` clamped to `1..=CAP`.
    Rate { asked: i32, used: i32 },
}

fn from_override(value: Option<&str>) -> Override {
    match value {
        None => Override::Unset,
        Some(v) => match v.trim().parse::<i32>() {
            Ok(asked) => Override::Rate { asked, used: asked.clamp(1, CAP) },
            Err(_) => Override::Bad(v.to_owned()),
        },
    }
}

/// The frame rate to hand CEF. The override is read once; the primary
/// display is asked every call (`refresh_hz` is `EnumDisplaySettingsW(None)`,
/// the primary display's current mode), so a pane shown again after the
/// primary display changed mode gets the new rate (`client.rs` calls this
/// from `open()` and `show()`). A pane moved to another monitor is not
/// followed: that is a per-window query, not wired. Logged on change.
pub(crate) fn frame_rate() -> i32 {
    static OVERRIDE: OnceLock<Option<i32>> = OnceLock::new();
    static LAST: AtomicI32 = AtomicI32::new(0);
    let over = *OVERRIDE.get_or_init(|| match from_override(std::env::var("SURYA_CEF_FPS").ok().as_deref()) {
        Override::Unset => None,
        Override::Bad(v) => {
            println!("browser: SURYA_CEF_FPS={v:?} is not a whole number; using the display's rate");
            None
        }
        Override::Rate { asked, used } => {
            if asked == used {
                println!("browser: frame rate {used} (SURYA_CEF_FPS)");
            } else {
                println!("browser: frame rate {used} (SURYA_CEF_FPS={asked} clamped to 1..={CAP})");
            }
            Some(used)
        }
    });
    if let Some(n) = over {
        return n;
    }
    let hz = refresh_hz();
    let n = pick(hz);
    if LAST.swap(n, Ordering::Relaxed) != n {
        println!("browser: frame rate {n} (display reports {hz} Hz)");
    }
    n
}

/// A reported rate to a cap: 119 and 120 both mean the 120 Hz screen; a
/// display that will not say gets 60.
fn pick(hz: i32) -> i32 {
    if hz >= 30 { hz.min(CAP) } else { FALLBACK }
}

/// The primary display's refresh rate, or 0 when the platform will not say.
#[cfg(windows)]
pub(crate) fn refresh_hz() -> i32 {
    use windows::Win32::Graphics::Gdi::{EnumDisplaySettingsW, DEVMODEW, ENUM_CURRENT_SETTINGS};
    let mut mode = DEVMODEW::default();
    mode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
    // SAFETY: a zeroed DEVMODEW of the declared size, filled by Windows.
    let ok = unsafe { EnumDisplaySettingsW(None, ENUM_CURRENT_SETTINGS, &mut mode) }.as_bool();
    if ok { mode.dmDisplayFrequency as i32 } else { 0 }
}

/// Linux and the Mac: the shell has no display query wired yet, so the
/// fallback applies. The Mac's is in haktui's `mac.rs` when a seat ports it.
#[cfg(not(windows))]
pub(crate) fn refresh_hz() -> i32 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_override_is_parsed_clamped_and_refused_on_purpose() {
        assert_eq!(from_override(None), Override::Unset);
        assert_eq!(from_override(Some("60")), Override::Rate { asked: 60, used: 60 });
        assert_eq!(from_override(Some(" 90 ")), Override::Rate { asked: 90, used: 90 });
        assert_eq!(from_override(Some("240")), Override::Rate { asked: 240, used: 120 });
        assert_eq!(from_override(Some("0")), Override::Rate { asked: 0, used: 1 });
        assert_eq!(from_override(Some("-5")), Override::Rate { asked: -5, used: 1 });
        assert_eq!(from_override(Some("fast")), Override::Bad("fast".into()));
        assert_eq!(from_override(Some("")), Override::Bad("".into()));
    }

    #[test]
    fn rate_follows_the_display_up_to_the_cap() {
        assert_eq!(pick(0), 60);
        assert_eq!(pick(60), 60);
        assert_eq!(pick(119), 119);
        assert_eq!(pick(120), 120);
        assert_eq!(pick(144), 120);
        assert_eq!(pick(240), 120);
    }
}
