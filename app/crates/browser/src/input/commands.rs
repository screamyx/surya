//! Keyboard and mouse shortcuts the pane acts on itself.

use gpui::{Keystroke, MouseButton};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Reload,
    ReloadIgnoringCache,
    Back,
    Forward,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    NewTab,
    CloseTab,
    NextTab,
    PreviousTab,
    /// Mobile view mode: phone viewport, touch, and the circle. Chrome's own
    /// chord for its device toolbar, ctrl+shift+m.
    ToggleMobile,
}

/// What a keystroke means to the browser chrome, if anything.
///
/// Returning `Some` means haktui acted and the key must **not** also reach the
/// page: F5 that both reloads and fires a keydown is not what Chrome does.
pub fn shortcut(ks: &Keystroke) -> Option<Command> {
    let m = &ks.modifiers;
    let key = ks.key.as_str();
    // ctrl on Windows and Linux, cmd on macOS. Accept either: the owner uses
    // both machines and a browser that ignores cmd+R on a Mac is not a Chrome.
    let ctrl = m.control || m.platform;

    if m.alt && !ctrl {
        return match key {
            "left" => Some(Command::Back),
            "right" => Some(Command::Forward),
            _ => None,
        };
    }
    if key == "f5" {
        return Some(if ctrl {
            Command::ReloadIgnoringCache
        } else {
            Command::Reload
        });
    }
    if !ctrl {
        return None;
    }
    match key {
        "r" => Some(if m.shift {
            Command::ReloadIgnoringCache
        } else {
            Command::Reload
        }),
        "t" => Some(Command::NewTab),
        "w" => Some(Command::CloseTab),
        // GPUI reports ctrl+tab as `tab`; shift picks the direction, as it does
        // in every application that has a tab strip.
        "tab" => Some(if m.shift {
            Command::PreviousTab
        } else {
            Command::NextTab
        }),
        // GPUI resolves ctrl+shift+= to "+" and clears shift, so both spellings
        // of zoom-in arrive here and both are Chrome's.
        "=" | "+" => Some(Command::ZoomIn),
        "-" | "_" => Some(Command::ZoomOut),
        "0" => Some(Command::ZoomReset),
        "m" if m.shift => Some(Command::ToggleMobile),
        _ => None,
    }
}

/// The two side buttons on a mouse. Chrome navigates with them.
pub fn mouse_shortcut(b: MouseButton) -> Option<Command> {
    use gpui::NavigationDirection;
    match b {
        MouseButton::Navigate(NavigationDirection::Back) => Some(Command::Back),
        MouseButton::Navigate(NavigationDirection::Forward) => Some(Command::Forward),
        _ => None,
    }
}

/// CEF's zoom level, where the scale is `1.2^level`. Chrome's own zoom-in step
/// is about 10 percent near 100 percent, which is half a level.
const ZOOM_STEP: f64 = 0.5;
/// Chrome stops at 25 percent and 500 percent. `log(0.25)/log(1.2)` is -7.6 and
/// `log(5)/log(1.2)` is 8.8.
const ZOOM_MIN: f64 = -7.6;
const ZOOM_MAX: f64 = 8.8;

pub fn zoom(level: f64, command: Command) -> f64 {
    match command {
        Command::ZoomIn => (level + ZOOM_STEP).min(ZOOM_MAX),
        Command::ZoomOut => (level - ZOOM_STEP).max(ZOOM_MIN),
        _ => 0.0,
    }
}

