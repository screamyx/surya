//! Tests for `skew_banner.rs`. Pure: no window, no shell.

use super::*;

use gpui::Hsla;

use crate::theme::{contrast_ratio, Appearance};
use surya_theme::{AccentSelection, SurfacePreference};

/// The palettes the app installs. `Theme::dark()` and `Theme::light()` are
/// hardcoded fallbacks that no live path uses, so a test that measured only
/// those could pass while the screen disagreed.
fn shipping_themes() -> Vec<Theme> {
    [(Appearance::Dark, "zeron-dark"), (Appearance::Light, "zeron-light")]
        .into_iter()
        .map(|(appearance, id)| {
            Theme::for_selection(
                appearance,
                id,
                AccentSelection::ThemeDefault,
                SurfacePreference::default(),
            )
        })
        .collect()
}

/// The finding: the banner floats over the transcript, so anything less than
/// an opaque fill lets the page read through it. On a Windows dark frame the
/// number words behind the banner were legible through the amber.
#[test]
fn the_banner_fill_is_opaque() {
    for theme in shipping_themes().into_iter().chain([Theme::dark(), Theme::light()]) {
        assert_eq!(
            fill(&theme).a,
            1.0,
            "{:?} banner fill is translucent, so the transcript reads through it",
            theme.appearance
        );
    }
}

/// The token it is built from is not opaque, which is the whole reason this
/// function exists. If `warning_wash` ever became opaque on its own, the
/// blend would be a no-op and this file could go.
#[test]
fn the_wash_it_is_built_from_is_translucent() {
    for theme in shipping_themes() {
        assert!(
            theme.warning_wash.a < 1.0,
            "{:?} warning_wash is opaque; the blend is now redundant",
            theme.appearance
        );
    }
}

/// Compositing, spelled out rather than borrowed, so this test cannot agree
/// with `fill` by sharing its arithmetic. Same shape as the one
/// `warning_wash_keeps_text_readable` uses in theme.rs.
fn over(top: Hsla, under: Hsla) -> Hsla {
    let (t, u) = (top.to_rgb(), under.to_rgb());
    let a = top.a;
    gpui::Rgba {
        r: t.r * a + u.r * (1.0 - a),
        g: t.g * a + u.g * (1.0 - a),
        b: t.b * a + u.b * (1.0 - a),
        a: 1.0,
    }
    .into()
}

/// Same colour on screen: every channel lands on the same 8-bit value. The
/// floats themselves are not equal, because the two paths reach the colour
/// through different arithmetic and the hue can differ in its last bit.
fn same_pixel(a: Hsla, b: Hsla) -> bool {
    let (x, y) = (a.to_rgb(), b.to_rgb());
    [(x.r, y.r), (x.g, y.g), (x.b, y.b), (x.a, y.a)]
        .into_iter()
        .all(|(p, q)| (p - q).abs() < 0.5 / 255.0)
}

fn channels(c: Hsla) -> [u8; 3] {
    let c = c.to_rgb();
    [c.r, c.g, c.b].map(|v| (v * 255.0).round() as u8)
}

/// Making it opaque must not change the colour a person sees where the
/// transcript behind is empty: those pixels are the ones the wash already
/// produced.
///
/// The plane is named here independently, and it is `surface`. That is not
/// what the token comments suggest (`bg` is commented "main panel"), so it is
/// worth stating why: the transcript is drawn straight onto the frost surface
/// with no plane of its own, and what paints there measures grey 13. The
/// banner already rendered as the wash over that plane, so the plane can be
/// solved for from a frame: with the measured fill, `surface` implies a
/// warning colour inside sRGB and `bg` implies a red channel of 284, which no
/// colour has.
#[test]
fn the_fill_is_the_wash_over_the_plane_the_transcript_sits_on() {
    for theme in shipping_themes() {
        let painted = fill(&theme);
        let on_surface = over(theme.warning_wash, theme.surface);
        assert!(
            same_pixel(painted, on_surface),
            "{:?} banner paints {:?}, the wash over surface is {:?}",
            theme.appearance,
            channels(painted),
            channels(on_surface)
        );
        // And it is not the other plane, or the banner would not match the
        // page behind it.
        let on_bg = over(theme.warning_wash, theme.bg);
        assert!(
            !same_pixel(painted, on_bg),
            "{:?} the wash over bg is also {:?}, so this test proves nothing",
            theme.appearance,
            channels(on_bg)
        );
    }
}

/// The banner's own text still clears AA on the fill, in both appearances.
/// The dismiss glyph is `text_muted`, the message is `text`.
#[test]
fn the_banner_text_clears_aa_on_the_fill() {
    for theme in shipping_themes().into_iter().chain([Theme::dark(), Theme::light()]) {
        let plane = fill(&theme);
        for (name, tone) in [("message", theme.text), ("dismiss", theme.text_muted)] {
            let ratio = contrast_ratio(tone, plane);
            assert!(
                ratio >= 4.5,
                "{:?} {name} is {ratio:.2}:1 on the banner fill, below AA",
                theme.appearance
            );
        }
    }
}
