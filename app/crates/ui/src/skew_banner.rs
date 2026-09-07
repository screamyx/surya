//! The fill behind the engine-skew banner.
//!
//! Its own file because `shell.rs` is far past the 500-line rule and this
//! carries a test; the banner itself stays in `shell.rs`
//! (`render_engine_skew_banner`).
//!
//! Two things about that banner are worth writing down where they will be
//! read, because both were wrong on a shipped build and neither is obvious
//! from the call site.
//!
//! `occlude()` does not occlude paint. gpui's `div.rs` maps it to
//! `occlude_mouse`, `HitboxBehavior::BlockMouse`. It is an input barrier and
//! nothing else, so it never stopped the transcript showing through; only an
//! opaque fill does that. Do not re-add it expecting otherwise.
//!
//! The banner box needs `min_w_0`. It asks for `max_w(720)` inside a centred
//! flex parent that is only as wide as the transcript column, about 663 px
//! once a side pane is docked at its default width. A flex item defaults to
//! `min-width: auto`, so without `min_w_0` the box keeps its content width,
//! overflows, and the centring splits the overflow both ways - which is why
//! the LEFT edge is the one that gets clipped and the message starts
//! mid-word.

use gpui::Hsla;

use crate::theme::{flatten, Theme};

/// What the banner paints behind its text.
///
/// The banner is absolutely positioned over the transcript, so it has to bring
/// its own opaque plane. `warning_wash` is not one: it is `warning` at wash
/// alpha (theme.rs `warning_wash_for`, 0.18 in dark, 0.10 in light), and the
/// token's own contract is a wash "over the panel" - its readability test
/// blends it over `bg` or `surface` before measuring. A floating banner has no
/// panel under it, so the wash alone let the transcript read straight through
/// the amber.
///
/// Blending against the plane the transcript sits on gives the colour the
/// token always intended, and gives it opaquely. Where the transcript behind
/// is empty the pixels are unchanged; where it has text, the text stops
/// showing through.
pub fn fill(theme: &Theme) -> Hsla {
    flatten(theme.warning_wash, theme.surface)
}

#[cfg(test)]
#[path = "skew_banner_tests.rs"]
mod tests;
