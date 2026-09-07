//! What background a LEFT NAV rail entry (Home, Needs you, Agents, Tasks,
//! Files) paints, at rest and under the pointer (E2E-UI-03).
//!
//! Not to be confused with `crate::rail`, which is the transcript's message
//! minimap.
//!
//! The rail used to apply the hover wash to every enabled row, selected ones
//! included, so pointing at the current entry REPLACED its selected colour
//! with the same plain grey any other row shows on hover. Selection in this
//! rail is carried by that one background, so hovering the row you are on
//! removed the only cue that you were on it.
//!
//! A pure module so the four (selected, enabled) cases can be asserted
//! without standing up a window; `shell.rs` is far past the 500-line rule
//! (decision 13) and does not grow for this.

use gpui::{hsla, Hsla};

use crate::theme::{flatten, Theme};

/// The two backgrounds one rail entry needs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowBackgrounds {
    /// Painted when the pointer is elsewhere. `None` means the rail shows
    /// through.
    pub rest: Option<Hsla>,
    /// Painted under the pointer. `None` means a disabled row, which must not
    /// react at all.
    pub hover: Option<Hsla>,
}

/// Rest and hover for a rail entry.
///
/// The rule the old code broke: hover may never take a selected row BACKWARDS.
/// A selected row deepens, keeping its hue; an unselected one takes the plain
/// hover wash; a disabled one does nothing.
pub fn row_backgrounds(theme: &Theme, selected: bool, enabled: bool) -> RowBackgrounds {
    let rest = selected.then_some(theme.element_active);
    let hover = match (enabled, selected) {
        (false, _) => None,
        (true, false) => Some(theme.element_hover),
        (true, true) => Some(deepen(theme.element_active, theme.element_hover)),
    };
    RowBackgrounds { rest, hover }
}

/// The selected wash, more of it. Hue and saturation are untouched - that is
/// the whole point, since the hue IS the selection cue.
///
/// A translucent plate deepens by stacking the hover wash's alpha on its own.
/// An opaque one has no alpha left to give, so the wash is composited over it
/// instead, which lands in the same direction.
fn deepen(active: Hsla, hover: Hsla) -> Hsla {
    if active.a < 1.0 {
        hsla(active.h, active.s, active.l, (active.a + hover.a).min(1.0))
    } else {
        flatten(hover, active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme() -> Theme {
        Theme::dark()
    }

    /// The finding itself: the pointer used to turn the selected row's colour
    /// into the ordinary hover grey, which is what every unselected row shows.
    #[test]
    fn hovering_the_selected_row_never_lands_on_the_unselected_hover() {
        let theme = theme();
        let selected = row_backgrounds(&theme, true, true);
        let plain = row_backgrounds(&theme, false, true);
        assert_ne!(
            selected.hover, plain.hover,
            "the selected row must not hover to the same wash an unselected one does"
        );
        assert_eq!(plain.hover, Some(theme.element_hover));
    }

    #[test]
    fn hover_deepens_the_selected_row_and_keeps_its_hue() {
        let theme = theme();
        let rest = theme.element_active;
        let hover = row_backgrounds(&theme, true, true)
            .hover
            .expect("an enabled row reacts");
        assert_eq!((hover.h, hover.s), (rest.h, rest.s), "the hue is the cue");
        assert!(
            hover.a >= rest.a,
            "hover must add to the selected wash, never take from it"
        );
        assert_ne!(hover, rest, "hover has to be visible");
    }

    #[test]
    fn an_unselected_row_rests_on_the_rail_and_a_disabled_one_never_reacts() {
        let theme = theme();
        assert_eq!(row_backgrounds(&theme, false, true).rest, None);
        assert_eq!(row_backgrounds(&theme, true, true).rest, Some(theme.element_active));
        assert_eq!(row_backgrounds(&theme, false, false).hover, None);
        assert_eq!(
            row_backgrounds(&theme, true, false).hover,
            None,
            "a disabled row shows its selection but does not answer the pointer"
        );
    }

    #[test]
    fn an_opaque_selected_plate_still_deepens() {
        // comet's own theme paints the selected rail row an opaque purple,
        // not a wash, so the alpha branch above would have nothing to add.
        let opaque = hsla(0.72, 0.21, 0.21, 1.0);
        let deeper = deepen(opaque, hsla(0.0, 0.0, 0.92, 0.11));
        assert_ne!(deeper, opaque, "an opaque plate has to move too");
    }
}
