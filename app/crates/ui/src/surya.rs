//! The surya look: the floating-panel geometry and the type scale that the
//! `surya-light` / `surya-dark` themes are drawn to.
//!
//! Colour lives in the theme crate. This module owns the two things a palette
//! cannot carry: **how far apart the panels sit** and **how big the text is**.
//! Both are tokens, so no call site writes a raw number and no call site writes
//! a hex.
//!
//! # The direction
//!
//! Floating rounded panels on a soft canvas. The rail is a card, the
//! conversation is a card, the right pane is a card, and the canvas (the
//! theme's `surface`) shows between them as a margin. Light first, dark as
//! good.
//!
//! # Depth without glass
//!
//! The browser pane may pin Zed's own gpui instead of comet's fork, and that
//! build has no backdrop blur and no transparent window. So every panel here
//! gets its depth from four things that work on any gpui: the canvas tone
//! under it, a hairline border, one soft shadow, and the gap. Glass, where the
//! fork still offers it, is an addition on top, never the mechanism.
//!
//! Two elevation levels, and only two. [`ELEVATION_PANEL`] is a panel resting
//! on the canvas; [`ELEVATION_FLOAT`] is something resting on a panel (the
//! composer pill, the file browser card). Anything higher is a popover and
//! belongs to comet's own overlay tokens.
//!
//! # What gpui cannot do
//!
//! There is no letter-spacing in gpui's text style, so the scale carries
//! hierarchy in size, weight and case alone. Where the taste doctrine asks for
//! loose tracking on an overline, this scale answers with size and weight.

use gpui::{FontFeatures, FontWeight, Hsla, Rems, Styled, px};

use crate::theme::Theme;
use crate::typography::ui_rems;

// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

/// Margin between the window edge and the outermost panel.
///
/// 16, twice the seam between two panels. It started at 10, and a render of
/// the layout at 1440x900 showed why that was wrong: at 10 against a seam of
/// 8, the window margin and the gap between two cards read as the same
/// measurement, so the panels looked evenly scattered rather than grouped
/// inside a frame. Outer must beat inner by enough to see.
pub const CANVAS_INSET: f32 = 16.0;

/// Gap between two adjacent panels. Reads as one seam, not two margins.
pub const PANEL_GAP: f32 = 8.0;

/// Corner radius of a floating panel. Wide, in the Raycast/CleanShot register.
///
/// Deliberately larger than comet's [`Theme::PANEL_RADIUS`] (10), which is the
/// radius of a panel *inside* another surface. A panel with a margin all round
/// needs the wider corner or it reads as a cropped rectangle.
pub const PANEL_RADIUS: f32 = 14.0;

/// Corner radius of a card that floats *over* a panel: the file browser over
/// the right pane. One step tighter than the panel it covers, so the stack
/// reads as layers rather than as one shape.
pub const FLOAT_RADIUS: f32 = 12.0;

/// Corner radius for inputs, chips and buttons. Comet's control radius,
/// unchanged: this is the tight end of the language, and re-picking it would
/// split the vocabulary across two files.
pub const CONTROL_RADIUS: f32 = Theme::CONTROL_RADIUS;

/// Inner padding of a floating panel's content, before its own rows pad
/// themselves. Outer beats inner: this is larger than any row gap.
pub const PANEL_PAD: f32 = 8.0;

/// The composer is a pill: its radius is half its height, so it stays a pill
/// as it grows with the text. Above this height it stops being a pill and
/// settles at [`PANEL_RADIUS`] — a 200px-tall capsule reads as a stadium, not
/// as an input.
pub const PILL_MAX_HEIGHT: f32 = 96.0;

/// The composer at rest: one line of text plus its control row. Measured from
/// comet's own collapsed pill.
pub const COMPOSER_RESTING_HEIGHT: f32 = 52.0;

/// The composer's radius, held constant as the pill grows.
///
/// [`pill_radius`] would hand a tall composer [`PANEL_RADIUS`], which is right
/// for a growing input in general and wrong for this one: the owner's sketch
/// puts a *pill* over the feed, and a composer that squares off as you type a
/// paragraph stops being the thing in the sketch. So the resting radius is
/// pinned. It resolves to 26, which is the number comet's composer already
/// used as a literal.
pub const COMPOSER_RADIUS: f32 = COMPOSER_RESTING_HEIGHT / 2.0;

/// Radius for a composer of the given height.
pub fn pill_radius(height: f32) -> f32 {
    if height <= PILL_MAX_HEIGHT {
        (height / 2.0).max(CONTROL_RADIUS)
    } else {
        PANEL_RADIUS
    }
}

// ---------------------------------------------------------------------------
// Elevation
// ---------------------------------------------------------------------------

/// The two elevation levels, and nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Elevation {
    /// A panel resting on the canvas.
    Panel,
    /// A card resting on a panel.
    Float,
}

pub const ELEVATION_PANEL: Elevation = Elevation::Panel;
pub const ELEVATION_FLOAT: Elevation = Elevation::Float;

impl Elevation {
    /// Blur radius in px. Light mode runs tighter: comet learned the hard way
    /// (`theme::glass_selected_shadows`) that a wide soft shadow on a bright
    /// field reads as a grey rim rather than as depth.
    fn blur(self, dark: bool) -> f32 {
        match (self, dark) {
            (Self::Panel, false) => 10.0,
            (Self::Panel, true) => 14.0,
            (Self::Float, false) => 16.0,
            (Self::Float, true) => 22.0,
        }
    }

    fn y_offset(self) -> f32 {
        match self {
            Self::Panel => 1.0,
            Self::Float => 4.0,
        }
    }

    /// Shadow alpha. Dark mode leans on the surface step instead — a black
    /// shadow on a near-black canvas is invisible and only costs fill rate.
    fn alpha(self, dark: bool) -> f32 {
        match (self, dark) {
            (Self::Panel, false) => 0.06,
            (Self::Panel, true) => 0.24,
            (Self::Float, false) => 0.10,
            (Self::Float, true) => 0.34,
        }
    }
}

/// One shadow, never a stack. Layered shadows sum into a rim on a light field.
pub fn shadow(elevation: Elevation, theme: &Theme) -> Vec<gpui::BoxShadow> {
    let dark = matches!(theme.appearance, crate::theme::Appearance::Dark);
    vec![gpui::BoxShadow {
        color: gpui::hsla(0.08, 0.10, 0.02, elevation.alpha(dark)),
        offset: gpui::point(px(0.0), px(elevation.y_offset())),
        blur_radius: px(elevation.blur(dark)),
        spread_radius: px(0.0),
        inset: false,
    }]
}

/// The fill of a floating panel. Its own function so a panel never reaches for
/// a raw theme field and never picks the canvas tone by accident.
pub fn panel_bg(theme: &Theme) -> Hsla {
    theme.bg
}

/// The fill of a card resting ON a panel: the file browser over the right
/// pane.
///
/// It is a separate tone, not the panel fill again, because the two
/// appearances separate layers differently. Light lifts with the border and
/// the shadow and can leave the fill alone; dark has to climb, and a dark
/// float painted in the panel's own tone disappears into it. That is exactly
/// what a 1440x900 render of the layout showed: in dark the file card was
/// invisible against the pane behind it while the same card read fine in
/// light. `surface_card` is the theme's own answer to that ladder.
pub fn float_bg(theme: &Theme) -> Hsla {
    theme.surface_card
}

/// The canvas the panels float on.
pub fn canvas_bg(theme: &Theme) -> Hsla {
    theme.surface
}

/// A floating panel: fill, hairline, wide radius, one shadow. Every panel in
/// the shell is built from this, so the three of them cannot drift apart.
///
/// The caller owns the size and the content. This owns the material.
pub fn panel(theme: &Theme, elevation: Elevation) -> gpui::Div {
    let radius = match elevation {
        Elevation::Panel => PANEL_RADIUS,
        Elevation::Float => FLOAT_RADIUS,
    };
    let fill = match elevation {
        Elevation::Panel => panel_bg(theme),
        Elevation::Float => float_bg(theme),
    };
    gpui::div()
        .bg(fill)
        .rounded(px(radius))
        .border_1()
        .border_color(theme.border)
        .shadow(shadow(elevation, theme))
        .overflow_hidden()
}

// ---------------------------------------------------------------------------
// Type scale
// ---------------------------------------------------------------------------

/// One step of the interface type scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypeStep {
    /// Size in px at the default 16px interface baseline.
    pub size: f32,
    /// Line height in px at that baseline.
    pub leading: f32,
    pub weight: FontWeight,
}

impl TypeStep {
    pub fn rems(self) -> Rems {
        ui_rems(self.size)
    }

    pub fn line_height(self) -> Rems {
        ui_rems(self.leading)
    }
}

/// Session titles and the one heading on an empty state. The only step that
/// is allowed to be big.
pub const DISPLAY: TypeStep = TypeStep {
    size: 28.0,
    leading: 32.0,
    weight: FontWeight::SEMIBOLD,
};

/// Panel headers: the rail's section labels, a pane's title.
pub const TITLE: TypeStep = TypeStep {
    size: 19.0,
    leading: 24.0,
    weight: FontWeight::SEMIBOLD,
};

/// Transcript prose and every ordinary row. Comet's existing base, kept: a
/// scale that moved the body size would reflow the whole app.
pub const BODY: TypeStep = TypeStep {
    size: 14.0,
    leading: 21.0,
    weight: FontWeight::NORMAL,
};

/// Buttons, chips, and row labels that must not look like prose.
pub const LABEL: TypeStep = TypeStep {
    size: 13.0,
    leading: 18.0,
    weight: FontWeight::MEDIUM,
};

/// Timestamps, counts, secondary meta.
pub const CAPTION: TypeStep = TypeStep {
    size: 11.0,
    leading: 15.0,
    weight: FontWeight::MEDIUM,
};

/// Paths, session ids, branch names. The monospace family carries these; the
/// size sits between caption and body so a path never outweighs the sentence
/// around it.
pub const MONO: TypeStep = TypeStep {
    size: 12.5,
    leading: 18.0,
    weight: FontWeight::NORMAL,
};

/// Display over body. The taste doctrine wants a dramatic jump, not a timid
/// one; this scale prints 2.00.
pub fn display_to_body_ratio() -> f32 {
    DISPLAY.size / BODY.size
}

/// Body over caption.
pub fn body_to_caption_ratio() -> f32 {
    BODY.size / CAPTION.size
}

/// Tabular figures, for anything that counts: unread badges, token counts,
/// durations, line numbers. Proportional digits make a number that updates in
/// place jitter sideways.
pub fn tabular() -> FontFeatures {
    FontFeatures(std::sync::Arc::new(vec![
        ("tnum".to_string(), 1),
        ("zero".to_string(), 1),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scale_has_real_contrast() {
        // Print the measured ratios: the brief asks for the number, not for a
        // claim that it is big enough.
        println!(
            "surya type scale: display={}px body={}px caption={}px mono={}px \
             display/body={:.2} body/caption={:.2}",
            DISPLAY.size,
            BODY.size,
            CAPTION.size,
            MONO.size,
            display_to_body_ratio(),
            body_to_caption_ratio(),
        );
        assert!(
            display_to_body_ratio() >= 1.8,
            "display/body {:.2} is a timid jump",
            display_to_body_ratio()
        );
        assert!(body_to_caption_ratio() >= 1.2);
    }

    /// Every step must be legible and every leading must clear its size.
    #[test]
    fn every_step_is_sane() {
        for (name, step) in [
            ("display", DISPLAY),
            ("title", TITLE),
            ("body", BODY),
            ("label", LABEL),
            ("caption", CAPTION),
            ("mono", MONO),
        ] {
            assert!(step.size >= 11.0, "{name} is smaller than 11px");
            assert!(
                step.leading > step.size,
                "{name}: leading {} does not clear size {}",
                step.leading,
                step.size
            );
        }
    }

    /// The radius language: tight on controls, wide on panels, pill on the
    /// composer until it grows out of being one.
    #[test]
    fn the_radius_language_is_ordered() {
        assert!(CONTROL_RADIUS < FLOAT_RADIUS);
        assert!(FLOAT_RADIUS < PANEL_RADIUS);
        assert_eq!(pill_radius(48.0), 24.0);
        assert_eq!(pill_radius(96.0), 48.0);
        assert_eq!(pill_radius(200.0), PANEL_RADIUS);
        // A collapsed input never goes below the control radius.
        assert_eq!(pill_radius(4.0), CONTROL_RADIUS);
        // The pinned composer radius must equal what the general rule would
        // give a composer at rest, or the token has silently drifted from the
        // shape it names.
        assert_eq!(COMPOSER_RADIUS, pill_radius(COMPOSER_RESTING_HEIGHT));
        assert_eq!(COMPOSER_RADIUS, 26.0);
    }

    /// Outer beats inner: the window margin is never tighter than the seam
    /// between two panels.
    #[test]
    fn spacing_hierarchy_holds() {
        // Not merely larger: large enough to read as a different measurement.
        assert!(
            CANVAS_INSET >= 2.0 * PANEL_GAP,
            "window margin {CANVAS_INSET} does not clearly beat the panel seam {PANEL_GAP}"
        );
        assert!(PANEL_GAP >= PANEL_PAD);
    }

    #[test]
    fn tabular_figures_ask_for_tnum() {
        let features = tabular();
        assert!(
            features
                .tag_value_list()
                .iter()
                .any(|(tag, value)| tag == "tnum" && *value == 1)
        );
    }
}
