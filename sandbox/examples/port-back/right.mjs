// Executable patch specimen for the question-row px-0.5 -> px-1 change.
// before is checked against the real native source; this module does not write it.
export const target = 'app/crates/ui/src/inbox/chrome.rs';
export const unit = 'px';
export const rootFontPx = 16;
export const fromClass = 'px-0.5';
export const toClass = 'px-1';
export const before = String.raw`pub fn row_line(theme: &Theme, first: bool) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .px(px(2.0))
        .py(px(10.0))
        .when(!first, |el| {
            el.border_t_1().border_color(theme.border)
        })
}`;
export const after = String.raw`pub fn row_line(theme: &Theme, first: bool) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .px(px(4.0))
        .py(px(10.0))
        .when(!first, |el| {
            el.border_t_1().border_color(theme.border)
        })
}`;
