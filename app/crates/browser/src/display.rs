//! CEF's paint-rate cap, from the display it is shown on.
//!
//! `windowless_frame_rate` had been 60 since haktui's first spike; the
//! owner's screen is 120 Hz and native Chrome paints at that. haktui
//! measured 2026-08-28: CEF capped at 60 gave 76 app frames on the scroll
//! test, at 120 it gave 122, and a still page paints nothing whatever the
//! cap, so idle cost did not move. Default is the primary display's refresh
//! rate, capped at 120; `SURYA_CEF_FPS=<n>` overrides.

use std::sync::OnceLock;

const FALLBACK: i32 = 60;
const CAP: i32 = 120;

/// The frame rate to hand CEF, read once.
pub(crate) fn frame_rate() -> i32 {
    static FPS: OnceLock<i32> = OnceLock::new();
    *FPS.get_or_init(|| {
        if let Some(n) = std::env::var("SURYA_CEF_FPS").ok().and_then(|v| v.trim().parse::<i32>().ok()) {
            let n = n.clamp(1, CAP);
            println!("browser: frame rate {n} (SURYA_CEF_FPS)");
            return n;
        }
        let hz = refresh_hz();
        let n = pick(hz);
        println!("browser: frame rate {n} (display reports {hz} Hz)");
        n
    })
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
    fn rate_follows_the_display_up_to_the_cap() {
        assert_eq!(pick(0), 60);
        assert_eq!(pick(60), 60);
        assert_eq!(pick(119), 119);
        assert_eq!(pick(120), 120);
        assert_eq!(pick(144), 120);
        assert_eq!(pick(240), 120);
    }
}
