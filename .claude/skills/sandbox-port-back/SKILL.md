---
name: sandbox-port-back
description: Carry an accepted surya sandbox screen diff and CHANGES.md back into the existing Rust GPUI render. Use when asked to apply a browser design change to the native app.
---

# Carry a screen change back to GPUI

## Worked example: carry the same side-padding change back

The accepted React change is `px-0.5` to `px-1` in the question-row render helper.

Wrong: [patch specimen](../../../sandbox/examples/port-back/wrong.mjs), `after`.

```rust
pub fn row_line(theme: &Theme, first: bool) -> Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .px(px(4.0))
        .py(px(10.0))
        .child(badge(theme, "Question", theme.warning))
        .child(div().flex_1().text_color(theme.text))
        .child(hint_chip(theme, "answer below"))
}
```

Right: [patch specimen](../../../sandbox/examples/port-back/right.mjs), `after`.

```rust
pub fn row_line(theme: &Theme, first: bool) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .px(px(4.0))
        .py(px(10.0))
        .when(!first, |el| {
            el.border_t_1().border_color(theme.border)
        })
}
```

Rebuilding the native row from JSX drops its first-row border handling for a
change that only needs one padding call.

## Load and execute

Read [CHANGES.md](../../../sandbox/CHANGES.md) with the accepted screen diff, then
follow [GUIDE: Port back, per screen, from the diff](../../../sandbox/GUIDE.md#port-back-per-screen-from-the-diff).
Use [One-to-one layout and paint](../../../sandbox/GUIDE.md#one-to-one-layout-and-paint)
and [whitelist.json](../../../sandbox/whitelist.json) for the method pair and the
record of fixed `px` versus scalable `ui_rems`. The worked patch records the native
`px` unit at the 16px reference root: only `.px(px(2.0))` becomes `.px(px(4.0))`.
Its lint checks the original Rust fragment and that exact edit, not Rust compilation.

Execute the native rebuild/check and screenshot steps in that procedure, including
[decision 28: Windows is the product](../../../docs/decisions.md#28-windows-is-the-product-mac-next-linux-is-a-test-bench-only).
Keep the browser captures and the rebuilt Windows screen together for review.

## Finish gate

From `sandbox/`, run in this order:

1. `npm run lint`
2. `npm test`
3. `npm run build`
4. Capture the screen beside its Windows reference frame, following
   [GUIDE: Reproduce proof](../../../sandbox/GUIDE.md#reproduce-proof).
