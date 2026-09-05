//! gpui input, translated into what CEF wants. Ported from haktui's
//! `input.rs` (2026-09-05), split by topic; no CEF types here, so the tests
//! run on every platform. `events.rs` fills CEF's structs from these.

mod commands;
mod keys;
mod mouse;
#[cfg(test)]
mod tests;

pub use commands::*;
pub use keys::*;
pub use mouse::*;

/// `cef_event_flags_t`. Checked against `cef::sys` at Windows compile time.
pub mod flags {
    pub const NONE: u32 = 0;
    pub const CAPS_LOCK_ON: u32 = 1 << 0;
    pub const SHIFT_DOWN: u32 = 1 << 1;
    pub const CONTROL_DOWN: u32 = 1 << 2;
    pub const ALT_DOWN: u32 = 1 << 3;
    pub const LEFT_MOUSE_BUTTON: u32 = 1 << 4;
    pub const MIDDLE_MOUSE_BUTTON: u32 = 1 << 5;
    pub const RIGHT_MOUSE_BUTTON: u32 = 1 << 6;
    /// Command on macOS, the Windows key here. GPUI calls it `platform`.
    pub const COMMAND_DOWN: u32 = 1 << 7;
    pub const NUM_LOCK_ON: u32 = 1 << 8;
    pub const IS_KEY_PAD: u32 = 1 << 9;
    pub const IS_LEFT: u32 = 1 << 10;
    pub const IS_RIGHT: u32 = 1 << 11;
    pub const ALTGR_DOWN: u32 = 1 << 12;
    pub const IS_REPEAT: u32 = 1 << 13;
    /// A trackpad delta that is already in pixels, not wheel notches.
    pub const PRECISION_SCROLLING_DELTA: u32 = 1 << 14;
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Left = 0,
    Middle = 1,
    Right = 2,
}

