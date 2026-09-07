//! Tests for `skew_banner.rs`. Pure: no window, no shell.

use super::*;

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

/// Making it opaque must not change the colour a person sees where the
/// transcript behind is empty. The blend is against the plane the transcript
/// sits on, so those pixels are the ones the wash already produced.
#[test]
fn the_fill_matches_the_wash_over_an_empty_transcript() {
    for theme in shipping_themes() {
        let painted = fill(&theme);
        let as_the_wash_read_before = flatten(theme.warning_wash, theme.surface);
        assert_eq!(painted, as_the_wash_read_before, "{:?}", theme.appearance);
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
