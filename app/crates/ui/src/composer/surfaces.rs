//! Float fills must cover the transcript when backdrop blur is unavailable.
use crate::theme::{Theme, flatten};
use gpui::Hsla;

pub(super) fn pill_background(theme: &Theme) -> Hsla {
    let tint = theme.input_glass_bg();
    if theme.is_frost() {
        tint
    } else {
        flatten(tint, theme.bg)
    }
}

/// Validation notices sit outside the pill's blur layer on every platform.
/// Preserve their subtle tint, composited over the canvas instead of text.
pub(super) fn notice_background(theme: &Theme, tint: Hsla) -> Hsla {
    flatten(tint, theme.bg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use surya_theme::SurfaceTreatment;

    #[test]
    fn opaque_composer_and_notices_cover_scrolled_text_in_both_themes() {
        for mut theme in [Theme::dark(), Theme::light()] {
            theme.surface_treatment = SurfaceTreatment::Opaque;
            for fill in [
                pill_background(&theme),
                notice_background(&theme, theme.danger.opacity(0.05)),
            ] {
                assert_eq!(fill.a, 1.0);
                let over_dark_text = flatten(fill, gpui::black());
                let over_light_text = flatten(fill, gpui::white());
                assert_eq!(
                    over_dark_text, over_light_text,
                    "transcript pixels must not change the surface"
                );
            }
        }
    }
}
