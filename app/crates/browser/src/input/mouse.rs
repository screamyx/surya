//! Buttons, modifiers, coordinates and the wheel.

use gpui::{Modifiers, MouseButton, Pixels, Point, ScrollDelta};

use super::{flags, Button};


/// One wheel notch is 120 units, the WHEEL_DELTA the whole Windows stack uses,
/// and Windows reports a notch as `wheel_scroll_lines` lines - 3 by default.
///
/// GPUI has already multiplied by that setting by the time we see the event, so
/// dividing by the default 3 keeps the owner's own "scroll this many lines"
/// setting: set it to 6 and each notch arrives as 6 lines and scrolls twice as
/// far, which is what he asked Windows for.
const PX_PER_LINE: f32 = 40.0;

/// Which mouse buttons are held, as CEF modifier bits. Kept as a bitmask
/// because CEF wants it on every event, including moves and wheels, and the
/// only thing that knows is us.
pub fn button_bit(b: MouseButton) -> Option<u32> {
    match b {
        MouseButton::Left => Some(flags::LEFT_MOUSE_BUTTON),
        MouseButton::Middle => Some(flags::MIDDLE_MOUSE_BUTTON),
        MouseButton::Right => Some(flags::RIGHT_MOUSE_BUTTON),
        // Back and forward are not buttons Chromium takes through this call.
        MouseButton::Navigate(_) => None,
    }
}

pub fn button_type(b: MouseButton) -> Option<Button> {
    match b {
        MouseButton::Left => Some(Button::Left),
        MouseButton::Middle => Some(Button::Middle),
        MouseButton::Right => Some(Button::Right),
        MouseButton::Navigate(_) => None,
    }
}

/// Modifier bits for a mouse event: the keyboard modifiers, plus whichever
/// buttons are currently held.
pub fn mouse_flags(m: &Modifiers, buttons_down: u32) -> u32 {
    modifier_flags(m) | buttons_down
}

pub(crate) fn modifier_flags(m: &Modifiers) -> u32 {
    let mut f = flags::NONE;
    if m.shift {
        f |= flags::SHIFT_DOWN;
    }
    if m.control {
        f |= flags::CONTROL_DOWN;
    }
    if m.alt {
        f |= flags::ALT_DOWN;
    }
    if m.platform {
        f |= flags::COMMAND_DOWN;
    }
    f
}

/// Window coordinates to view coordinates.
///
/// Both are DIPs: GPUI's `Pixels` are logical, and CEF is told the same scale
/// factor through `screen_info`, so this is a subtraction and not a conversion.
/// It is allowed to go negative and does, every time a drag that started inside
/// the panel leaves it, and Chromium handles that correctly.
pub fn view_point(pos: Point<Pixels>, origin: (i32, i32)) -> (i32, i32) {
    (
        f32::from(pos.x).round() as i32 - origin.0,
        f32::from(pos.y).round() as i32 - origin.1,
    )
}

/// A scroll wheel event, as CEF's `(delta_x, delta_y, modifiers)`.
///
/// Two things this does that are not obvious:
///
/// **Pixel deltas are marked as such.** A trackpad gives GPUI
/// `ScrollDelta::Pixels`, and passing those through as if they were wheel
/// notches scrolls a page by hundreds of lines. `PRECISION_SCROLLING_DELTA` is
/// how CEF is told which it is holding.
///
/// **Shift is dropped when the delta is already horizontal.** GPUI's Windows
/// backend puts a shift-wheel on the x axis itself. Passing `SHIFT_DOWN` on top
/// of that risks a second swap inside Chromium, which would turn a horizontal
/// scroll back into a vertical one. Dropping the bit is correct whether or not
/// Chromium would have swapped, which is why it is done rather than measured.
pub fn wheel(delta: ScrollDelta, m: &Modifiers) -> (i32, i32, u32) {
    let (dx, dy, precise) = match delta {
        ScrollDelta::Lines(p) => (p.x * PX_PER_LINE, p.y * PX_PER_LINE, false),
        ScrollDelta::Pixels(p) => (f32::from(p.x), f32::from(p.y), true),
    };
    let mut f = modifier_flags(m);
    if precise {
        f |= flags::PRECISION_SCROLLING_DELTA;
    }
    if dx != 0.0 {
        f &= !flags::SHIFT_DOWN;
    }
    (dx.round() as i32, dy.round() as i32, f)
}
