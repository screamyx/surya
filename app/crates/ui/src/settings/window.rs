//! Where the window was left, and whether that is still somewhere it can go.
//!
//! Every launch used to open the same centred 1320x880 window whatever the
//! person did last (E2E-WIN-02: closed at 1094x1447 at (52,0), reopened at
//! 1336x888 at (1052,276), four launches in a row on the same default). This
//! module holds the saved shape and the one decision that matters when it is
//! read back: a rect saved on a monitor that is no longer there must not open
//! a window nobody can reach.
//!
//! Everything here is pure. The window itself is `crate::open_main_window`'s
//! business; this file only answers "what shape, and is it safe".

use gpui::{px, size, Bounds, Pixels, Point, Size, WindowBounds};
use serde::{Deserialize, Serialize};

/// How the window was last left.
///
/// The rect is always the restore rect, the one the window returns to when it
/// is un-maximized, because that is what the platform reports for a maximized
/// window (gpui platform.rs 1885-1887: "The bounds provided here represent the
/// restore size of the window"). Storing it means un-maximizing after a
/// restart lands where it did before, not on a default.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowPlacement {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// Reopen maximized. A person who quit a maximized window expects it back
    /// that way.
    #[serde(default)]
    pub maximized: bool,
}

/// The narrowest and shortest slice of the window's top edge that still
/// counts as reachable: enough of the drag strip to put a pointer on and pull
/// the window back into view.
const REACHABLE_W: f32 = 120.0;
const REACHABLE_H: f32 = 30.0;

impl WindowPlacement {
    /// What to save for the state gpui reports.
    ///
    /// Fullscreen is deliberately saved as not-maximized: an app that reopens
    /// fullscreen with its own chrome hidden gives a person no visible way
    /// back out, which is a worse failure than reopening windowed at the same
    /// rect. Maximized does come back maximized.
    pub fn of(bounds: WindowBounds) -> Self {
        let (rect, maximized) = match bounds {
            WindowBounds::Windowed(rect) => (rect, false),
            WindowBounds::Maximized(rect) => (rect, true),
            WindowBounds::Fullscreen(rect) => (rect, false),
        };
        Self {
            x: f32::from(rect.origin.x),
            y: f32::from(rect.origin.y),
            width: f32::from(rect.size.width),
            height: f32::from(rect.size.height),
            maximized,
        }
    }

    fn rect(&self) -> Bounds<Pixels> {
        Bounds {
            origin: Point { x: px(self.x), y: px(self.y) },
            size: Size { width: px(self.width), height: px(self.height) },
        }
    }

    /// A saved rect that survived a hand-edited settings file and a machine
    /// whose displays changed. Anything that fails this opens on the default
    /// instead.
    fn is_sane(&self) -> bool {
        [self.x, self.y, self.width, self.height].iter().all(|v| v.is_finite())
            && self.width >= REACHABLE_W
            && self.height >= REACHABLE_H
    }

    /// Whether some display still shows enough of the window's top edge to
    /// grab it.
    ///
    /// Overlapping a display at all is not enough: a window one pixel onto
    /// the screen is a window nobody can move. The test is the drag strip,
    /// against each display on its own, because two displays that each show a
    /// sliver do not add up to a grabbable one.
    fn is_reachable(&self, displays: &[Bounds<Pixels>]) -> bool {
        let rect = self.rect();
        let strip = Bounds { origin: rect.origin, size: size(rect.size.width, px(REACHABLE_H)) };
        displays.iter().any(|display| {
            let seen = strip.intersect(display);
            seen.size.width >= px(REACHABLE_W) && seen.size.height >= px(REACHABLE_H)
        })
    }
}

/// The bounds to open with, or `None` to use the centred default.
///
/// `None` in, `None` out: a first run has nothing saved. A saved placement
/// that is off-screen or nonsense also gives `None`, so the caller has one
/// fallback path and not three.
pub fn restore(saved: Option<WindowPlacement>, displays: &[Bounds<Pixels>]) -> Option<WindowBounds> {
    let saved = saved?;
    if !saved.is_sane() || !saved.is_reachable(displays) {
        return None;
    }
    Some(if saved.maximized {
        WindowBounds::Maximized(saved.rect())
    } else {
        WindowBounds::Windowed(saved.rect())
    })
}

#[cfg(test)]
#[path = "window_tests.rs"]
mod tests;
