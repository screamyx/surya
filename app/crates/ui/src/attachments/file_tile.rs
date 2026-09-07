use crate::theme::Theme;
use gpui::{prelude::*, *};

/// A text attachment has a filename, not an empty image preview. The full
/// name stays available on hover when the compact strip truncates it.
pub fn file_tile(
    id: SharedString,
    name: SharedString,
    width: f32,
    height: f32,
    theme: &Theme,
) -> AnyElement {
    let label = name.clone();
    div()
        .id(id)
        .w(px(width))
        .h(px(height))
        .flex_none()
        .rounded(px(7.0))
        .border_1()
        .border_color(theme.border_strong)
        .bg(theme.surface)
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(4.0))
        .px(px(4.0))
        .overflow_hidden()
        .tooltip(move |_, cx| cx.new(|_| FileNameTooltip(name.clone())).into())
        .child(
            crate::icons::icon(crate::icons::DOCUMENT)
                .size(px(18.0))
                .text_color(theme.text_muted),
        )
        .child(
            div()
                .w_full()
                .text_ellipsis()
                .text_center()
                .text_size(crate::typography::ui_rems(10.0))
                .text_color(theme.text)
                .child(label),
        )
        .into_any_element()
}

struct FileNameTooltip(SharedString);
impl Render for FileNameTooltip {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx);
        div()
            .px(px(8.0))
            .py(px(6.0))
            .max_w(px(320.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(theme.border_strong)
            .bg(theme.surface_raised)
            .shadow_md()
            .text_size(crate::typography::ui_rems(11.0))
            .text_color(theme.text)
            .child(self.0.clone())
    }
}
