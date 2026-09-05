//! The two panels every failure lands on: the per-component fallback box
//! and the card-level diagnostics.

use super::*;

impl<'a> Renderer<'a> {
    /// The never-crash answer to anything the renderer cannot draw.
    pub(crate) fn fallback_box(&self, label: &str) -> AnyElement {
        let theme = self.theme;
        div()
            .w_full()
            .min_w_0()
            .rounded(px(theme.control_radius))
            .border_1()
            .border_dashed()
            .border_color(theme.border_strong)
            .px(px(10.0))
            .py(px(8.0))
            .text_size(rems(12.0 / 16.0))
            .line_height(px(16.0))
            .font_family(theme.font_mono.clone())
            .text_color(theme.text_muted)
            .child(SharedString::from(format!("Unsupported: {label}")))
            .into_any_element()
    }

    /// Parse diagnostics: the whole body when there is no root (or the
    /// budget is spent, with `headline`), a quiet footer otherwise.
    pub(crate) fn diagnostics(&self, whole: bool, headline: Option<String>) -> AnyElement {
        let theme = self.theme;
        let mut el = div()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .px(px(CARD_PADDING))
            .py(px(if whole { CARD_PADDING } else { 8.0 }))
            .text_size(rems(12.0 / 16.0))
            .line_height(px(16.0))
            .font_family(theme.font_mono.clone())
            .text_color(theme.text_muted);
        if whole {
            el = el.child(
                div()
                    .font_family(theme.font_sans.clone())
                    .text_size(rems(13.0 / 16.0))
                    .text_color(theme.danger)
                    .child("This card could not be drawn"),
            );
        } else {
            el = el.border_t_1().border_color(theme.border);
        }
        el.children(
            headline
                .into_iter()
                .chain(self.card.errors.iter().cloned())
                .map(|e| div().child(SharedString::from(e))),
        )
        .into_any_element()
    }
}
