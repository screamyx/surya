//! Leaf components: Text, Image, Divider, TextField, CheckBox.


use gpui::prelude::*;
use gpui::{
    AnyElement, ClickEvent, FontWeight, ImageSource, KeyDownEvent, ObjectFit, SharedString, div,
    img, px, rems, svg,
};
use serde_json::Value;
use serde_json::json;

use crate::data::resolve_string;
use crate::images::ResolvedImage;
use crate::inline::{InkStyle, parse_inline, styled_text};
use crate::model::*;
use crate::render::{CARD_PADDING, Ctx, Renderer};
use crate::state::{CardEvent, CardState};
use crate::theme::CardTheme;

/// Header images bleed to the plate edge, like the mockup's cover photo.
const HEADER_IMAGE_HEIGHT: f32 = 176.0;
/// Longest text a single Text or field shows; the rest is clipped with an
/// ellipsis so a 100 MB string cannot become a 100 MB layout.
pub const MAX_TEXT_CHARS: usize = 4_000;
/// Labels (tab titles, checkbox labels) are shorter still.
pub const MAX_LABEL_CHARS: usize = 200;

/// `s` cut to `max` chars with an ellipsis when it was longer.
pub fn clip(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((ix, _)) => format!("{}…", &s[..ix]),
        None => s.to_owned(),
    }
}

pub(crate) fn text(
    r: &Renderer,
    ctx: &Ctx,
    text: &Dynamic<String>,
    variant: TextVariant,
) -> AnyElement {
    let theme = r.theme;
    let value = clip(&resolve_string(&r.state.data, &ctx.scope, text), MAX_TEXT_CHARS);
    let runs = parse_inline(&value);
    let (size, line, weight, color) = match variant {
        TextVariant::H1 => (24.0, 30.0, FontWeight::SEMIBOLD, theme.text),
        TextVariant::H2 => (20.0, 26.0, FontWeight::SEMIBOLD, theme.text),
        TextVariant::H3 => (17.0, 24.0, FontWeight::SEMIBOLD, theme.text),
        TextVariant::H4 => (15.0, 22.0, FontWeight::SEMIBOLD, theme.text),
        TextVariant::H5 => (14.0, 20.0, FontWeight::MEDIUM, theme.text),
        TextVariant::Body => (14.0, 20.0, FontWeight::NORMAL, theme.text),
        TextVariant::Caption => (12.0, 16.0, FontWeight::NORMAL, theme.text_muted),
    };
    let color = ctx.ink.unwrap_or(color);
    let styled = styled_text(
        &runs,
        &InkStyle {
            sans: theme.font_sans.clone(),
            mono: theme.font_mono.clone(),
            color,
            code_color: if ctx.ink.is_some() { color } else { theme.code_text },
            code_wash: theme.code_wash,
            weight,
        },
    );
    div()
        .min_w_0()
        .text_size(rems(size / 16.0))
        .line_height(px(line))
        .child(styled)
        .into_any_element()
}

pub(crate) fn image(
    r: &Renderer,
    ctx: &Ctx,
    url: &Dynamic<String>,
    fit: ImageFit,
    variant: ImageVariant,
    description: Option<&Dynamic<String>>,
) -> AnyElement {
    let theme = r.theme;
    let url = resolve_string(&r.state.data, &ctx.scope, url);
    // The host's policy decides what may load: no fetch happens for a URL
    // it did not clear (a card is model-authored, and gpui fetches on
    // render with no click).
    let source: ImageSource = match r.state.image(&url, r.policy) {
        ResolvedImage::Source(source) => source,
        ResolvedImage::Placeholder(host) => {
            return r.fallback_box(&format!("remote image from {host} (remote images are off)"));
        }
        ResolvedImage::Denied(reason) => return r.fallback_box(&reason),
    };
    let fit = match fit {
        ImageFit::Contain => ObjectFit::Contain,
        ImageFit::Cover => ObjectFit::Cover,
        ImageFit::Fill => ObjectFit::Fill,
        ImageFit::None => ObjectFit::None,
        ImageFit::ScaleDown => ObjectFit::ScaleDown,
    };
    let alt = description
        .map(|d| clip(&resolve_string(&r.state.data, &ctx.scope, d), MAX_LABEL_CHARS))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| clip(&url, MAX_LABEL_CHARS));
    let wash = theme.element_hover;
    let muted = theme.text_faint;
    let mono = theme.font_mono.clone();
    let fallback_alt = SharedString::from(alt);
    let mut el = img(source)
        .object_fit(fit)
        .flex_none()
        .bg(wash)
        .with_fallback(move || {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .px(px(8.0))
                .text_size(rems(11.0 / 16.0))
                .font_family(mono.clone())
                .text_color(muted)
                .child(fallback_alt.clone())
                .into_any_element()
        });
    el = match variant {
        ImageVariant::Icon => el.size(px(24.0)).rounded(px(4.0)),
        ImageVariant::Avatar => el.size(px(40.0)).rounded_full(),
        ImageVariant::SmallFeature => el.size(px(100.0)).rounded(px(theme.control_radius)),
        ImageVariant::MediumFeature => el
            .w_full()
            .h(px(200.0))
            .rounded(px(theme.control_radius)),
        ImageVariant::LargeFeature => el
            .w_full()
            .h(px(280.0))
            .rounded(px(theme.control_radius)),
        ImageVariant::Header => el.w_full().h(px(HEADER_IMAGE_HEIGHT)),
    };
    match variant {
        // Bleed to the plate edge; the plate clips the corners.
        ImageVariant::Header if ctx.depth <= 2 => div()
            .mx(px(-CARD_PADDING))
            .mt(px(-CARD_PADDING))
            .mb(px(4.0))
            .child(el)
            .into_any_element(),
        _ => el.into_any_element(),
    }
}

pub(crate) fn divider(theme: &CardTheme, axis: Axis) -> AnyElement {
    match axis {
        Axis::Horizontal => div()
            .w_full()
            .h(px(1.0))
            .flex_none()
            .bg(theme.border)
            .into_any_element(),
        Axis::Vertical => div()
            .w(px(1.0))
            .min_h(px(16.0))
            .h_full()
            .flex_none()
            .bg(theme.border)
            .into_any_element(),
    }
}

pub(crate) fn text_field(
    r: &Renderer,
    ctx: &Ctx,
    c: &Component,
    label: &Dynamic<String>,
    value: Option<&Dynamic<String>>,
    variant: TextFieldVariant,
) -> AnyElement {
    let theme = r.theme;
    let label = clip(&resolve_string(&r.state.data, &ctx.scope, label), MAX_LABEL_CHARS);
    let binding = CardState::binding_for(&c.id, &ctx.scope, value);
    let current = clip(&r.state.read_string(&binding, value, &ctx.scope), MAX_TEXT_CHARS);
    let focused = r.state.focused.as_deref() == Some(c.id.as_str());
    let shown = match variant {
        TextFieldVariant::Obscured => "•".repeat(current.chars().count()),
        _ => current.clone(),
    };
    let on_focus = r.on_event.clone();
    let focus_id = c.id.clone();
    let mut field = div()
        .id(r.eid(ctx, &c.id))
        .w_full()
        .min_w_0()
        .min_h(px(match variant {
            TextFieldVariant::LongText => 64.0,
            _ => 32.0,
        }))
        .flex()
        .flex_row()
        .items_start()
        .px(px(10.0))
        .py(px(6.0))
        .rounded(px(theme.control_radius))
        .border_1()
        .border_color(if focused {
            theme.accent
        } else {
            theme.border
        })
        .bg(theme.input_bg)
        .cursor_text()
        .text_size(rems(14.0 / 16.0))
        .line_height(px(20.0))
        .text_color(theme.text)
        .on_click(move |_: &ClickEvent, window, cx| {
            on_focus(CardEvent::Focus(Some(focus_id.clone())), window, cx)
        });
    if shown.is_empty() {
        field = field.child(
            div()
                .text_color(theme.text_faint)
                .child(SharedString::from(label.clone())),
        );
    } else {
        field = field.child(div().min_w_0().child(SharedString::from(shown)));
    }
    if focused {
        field = field.child(
            div()
                .w(px(1.0))
                .h(px(18.0))
                .mt(px(1.0))
                .flex_none()
                .bg(theme.text),
        );
        if let Some(handle) = &r.state.focus {
            let on_key = r.on_event.clone();
            let numeric = variant == TextFieldVariant::Number;
            field = field
                .track_focus(handle)
                .on_key_down(move |ev: &KeyDownEvent, window, cx| {
                    let ks = &ev.keystroke;
                    let mut next = current.clone();
                    match ks.key.as_str() {
                        "backspace" => {
                            next.pop();
                        }
                        "enter" | "escape" | "tab" => {
                            on_key(CardEvent::Focus(None), window, cx);
                            return;
                        }
                        _ => {
                            let Some(ch) = ks.key_char.as_deref() else {
                                return;
                            };
                            if ks.modifiers.platform || ks.modifiers.control {
                                return;
                            }
                            if numeric && !ch.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-') {
                                return;
                            }
                            next.push_str(ch);
                        }
                    }
                    on_key(
                        CardEvent::SetValue {
                            binding: binding.clone(),
                            value: json!(next),
                        },
                        window,
                        cx,
                    );
                });
        }
    }
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .w_full()
        .min_w_0()
        .child(
            div()
                .text_size(rems(12.0 / 16.0))
                .line_height(px(16.0))
                .text_color(theme.text_muted)
                .child(SharedString::from(label)),
        )
        .child(field)
        .into_any_element()
}

pub(crate) fn check_box(
    r: &Renderer,
    ctx: &Ctx,
    c: &Component,
    label: &Dynamic<String>,
    value: &Dynamic<bool>,
) -> AnyElement {
    let theme = r.theme;
    let label = clip(&resolve_string(&r.state.data, &ctx.scope, label), MAX_LABEL_CHARS);
    let binding = CardState::binding_for_bool(&c.id, &ctx.scope, value);
    let checked = r.state.read_bool(&binding, value, &ctx.scope);
    let on_event = r.on_event.clone();
    let tick = div()
        .size(px(16.0))
        .flex_none()
        .rounded(px(4.0))
        .border_1()
        .border_color(if checked {
            theme.solid
        } else {
            theme.border_strong
        })
        .bg(if checked {
            theme.solid
        } else {
            gpui::transparent_black()
        })
        .flex()
        .items_center()
        .justify_center()
        .when(checked, |el| match &theme.check_icon {
            Some(icon) => el.child(
                svg()
                    .path(icon.clone())
                    .size(px(12.0))
                    .flex_none()
                    .text_color(theme.on_solid),
            ),
            None => el.child(div().size(px(8.0)).rounded(px(2.0)).bg(theme.on_solid)),
        });
    div()
        .id(r.eid(ctx, &c.id))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .min_w_0()
        .cursor_pointer()
        .text_size(rems(14.0 / 16.0))
        .line_height(px(20.0))
        .text_color(ctx.ink.unwrap_or(theme.text))
        .on_click(move |_: &ClickEvent, window, cx| {
            on_event(
                CardEvent::SetValue {
                    binding: binding.clone(),
                    value: json!(!checked),
                },
                window,
                cx,
            )
        })
        .child(tick)
        .child(div().min_w_0().child(SharedString::from(label)))
        .into_any_element()
}

/// Bar height cap for [`bar_chart`].
const BAR_MAX_HEIGHT: f32 = 48.0;
/// Bars drawn at most; a longer series is clipped to its head.
const BAR_MAX_ITEMS: usize = 60;

/// surya's BarChart extension: one bar per item of the bound array, scaled
/// to `max` or the largest value, labels underneath. Items are numbers, or
/// objects read through `value_key` / `label_key`.
pub(crate) fn bar_chart(
    r: &Renderer,
    ctx: &Ctx,
    values: &Dynamic<String>,
    value_key: Option<&str>,
    label_key: Option<&str>,
    max: Option<f64>,
) -> AnyElement {
    let theme = r.theme;
    let Some(path) = values.path() else {
        return r.fallback_box("BarChart needs a values path");
    };
    let abs = ctx.scope.absolute(path);
    let items: Vec<(f64, String)> = r
        .state
        .data
        .get(&abs)
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .take(BAR_MAX_ITEMS)
                .map(|item| {
                    let value = match (item, value_key) {
                        (Value::Number(n), _) => n.as_f64().unwrap_or(0.0),
                        (Value::Object(o), Some(k)) => o.get(k).and_then(Value::as_f64).unwrap_or(0.0),
                        (Value::Object(o), None) => o.get("value").and_then(Value::as_f64).unwrap_or(0.0),
                        _ => 0.0,
                    };
                    let label = match (item, label_key) {
                        (Value::Object(o), Some(k)) => o.get(k).map(crate::data::value_to_string),
                        (Value::Object(o), None) => o.get("label").map(crate::data::value_to_string),
                        _ => None,
                    }
                    .unwrap_or_default();
                    (value.max(0.0), clip(&label, 12))
                })
                .collect()
        })
        .unwrap_or_default();
    if items.is_empty() {
        return r.fallback_box(&format!("BarChart: no data at {abs}"));
    }
    let scale = max
        .filter(|m| *m > 0.0)
        .unwrap_or_else(|| items.iter().map(|(v, _)| *v).fold(0.0, f64::max))
        .max(f64::EPSILON);
    div()
        .flex()
        .flex_row()
        .items_end()
        .gap(px(6.0))
        .w_full()
        .min_w_0()
        // One budget unit per bar (a bar is a small subtree), and the bar
        // never leaves its lane: a model-supplied `max` below a value clamps.
        .children(items.into_iter().take_while(|_| r.budget.take()).map(|(value, label)| {
            let h = ((value / scale) as f32 * BAR_MAX_HEIGHT).clamp(3.0, BAR_MAX_HEIGHT);
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .items_center()
                .gap(px(4.0))
                .child(
                    div()
                        .w_full()
                        .h(px(BAR_MAX_HEIGHT))
                        .flex()
                        .items_end()
                        .child(div().w_full().h(px(h)).rounded(px(3.0)).bg(theme.solid)),
                )
                .child(
                    div()
                        .text_size(rems(11.0 / 16.0))
                        .line_height(px(14.0))
                        .text_color(theme.text_muted)
                        .whitespace_nowrap()
                        .child(SharedString::from(label)),
                )
        }))
        .into_any_element()
}
