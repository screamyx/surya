//! The GPUI renderer: containers, buttons, tabs, nested cards, and the
//! fallback box. Leaves live in [`crate::leaf`]. Everything is painted with
//! [`CardTheme`] tokens the host supplied — no color literal lives here.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{AnyElement, App, ClickEvent, FontWeight, Hsla, SharedString, Window, div, px, rems};

use crate::budget::{Budget, expand_children};
use crate::data::{Scope, resolve_string};
use crate::images::ImagePolicy;
use crate::model::*;
use crate::state::{CardEvent, CardState};
use crate::theme::CardTheme;

/// The host's sink for card events.
pub type OnEvent = Rc<dyn Fn(CardEvent, &mut Window, &mut App)>;

/// Nesting cap: a self-referencing tree stops here with a labelled box.
pub const MAX_DEPTH: usize = 24;
/// Template instantiation cap per list.
pub const MAX_TEMPLATE_ITEMS: usize = 200;
/// Card width cap inside the transcript column.
pub const MAX_CARD_WIDTH: f32 = 600.0;
/// Inner padding of the card plate.
pub const CARD_PADDING: f32 = 14.0;
/// Gap between siblings in a Row or Column.
pub const STACK_GAP: f32 = 8.0;

/// One render pass over a card. Borrow the card, its state, and the theme;
/// the closure receives every click and keystroke.
pub struct Renderer<'a> {
    pub card: &'a Card,
    pub state: &'a CardState,
    pub theme: &'a CardTheme,
    /// Where this card's images may load from (host-owned).
    pub policy: &'a ImagePolicy,
    /// Element cap for this render pass; a fresh [`Budget::default`] per row.
    pub budget: Budget,
    /// Prefix for every stateful element id — unique per transcript row.
    pub key: SharedString,
    pub on_event: OnEvent,
}

/// Per-branch render context.
#[derive(Clone)]
pub(crate) struct Ctx {
    pub scope: Scope,
    pub depth: usize,
    /// Text color forced by an ancestor (a primary button's label).
    pub ink: Option<Hsla>,
    /// Element-id discriminator for template instances.
    pub instance: String,
}

impl Ctx {
    fn root() -> Self {
        Ctx {
            scope: Scope::root(),
            depth: 0,
            ink: None,
            instance: String::new(),
        }
    }

    fn deeper(&self) -> Self {
        Ctx {
            depth: self.depth + 1,
            ..self.clone()
        }
    }
}

impl<'a> Renderer<'a> {
    /// The whole card: a bordered plate on the transcript column, the root
    /// tree inside, diagnostics when the tree is unusable.
    pub fn render(&self) -> AnyElement {
        let theme = self.theme;
        let mut frame = div()
            .w_full()
            .max_w(px(MAX_CARD_WIDTH))
            .flex()
            .flex_col()
            .min_w_0()
            .rounded(px(theme.radius))
            .border_1()
            .border_color(theme.border)
            .bg(theme.surface)
            .overflow_hidden()
            .font_family(theme.font_sans.clone())
            .text_color(theme.text);
        match self.card.root() {
            Some(root) => {
                let ctx = Ctx::root();
                // A root Card IS the plate: draw its child straight in
                // rather than a border inside a border.
                let body = match &root.kind {
                    ComponentKind::Card { child } => self.component(child, &ctx.deeper()),
                    _ => self.component(ROOT_ID, &ctx),
                };
                if self.budget.exceeded() {
                    // A runaway tree (template fan-out) is not drawn at all:
                    // the panel says why instead of a half card.
                    drop(body);
                    frame = frame.child(self.diagnostics(
                        true,
                        Some(format!(
                            "card exceeds the element budget ({} elements)",
                            self.budget.cap()
                        )),
                    ));
                } else {
                    frame = frame.child(
                        div()
                            .w_full()
                            .min_w_0()
                            .p(px(CARD_PADDING))
                            .flex()
                            .flex_col()
                            .child(body),
                    );
                    if !self.card.errors.is_empty() {
                        frame = frame.child(self.diagnostics(false, None));
                    }
                }
            }
            None => frame = frame.child(self.diagnostics(true, None)),
        }
        frame.into_any_element()
    }

    pub(crate) fn eid(&self, ctx: &Ctx, id: &str) -> SharedString {
        SharedString::from(format!("{}/{}{}", self.key, ctx.instance, id))
    }

    pub(crate) fn component(&self, id: &str, ctx: &Ctx) -> AnyElement {
        if ctx.depth > MAX_DEPTH {
            return self.fallback_box(&format!("nesting too deep at \"{id}\""));
        }
        if !self.budget.take() {
            // Cheap and non-recursive: the whole card is replaced by the
            // budget panel in `render`, this box never shows.
            return gpui::Empty.into_any_element();
        }
        let Some(c) = self.card.get(id) else {
            return self.fallback_box(&format!("missing component \"{id}\""));
        };
        let ctx = ctx.deeper();
        let el = match &c.kind {
            ComponentKind::Text { text, variant } => crate::leaf::text(self, &ctx, text, *variant),
            ComponentKind::Image {
                url,
                fit,
                variant,
                description,
            } => crate::leaf::image(self, &ctx, url, *fit, *variant, description.as_ref()),
            ComponentKind::Divider { axis } => crate::leaf::divider(self.theme, *axis),
            ComponentKind::TextField {
                label,
                value,
                variant,
            } => crate::leaf::text_field(self, &ctx, c, label, value.as_ref(), *variant),
            ComponentKind::CheckBox { label, value } => {
                crate::leaf::check_box(self, &ctx, c, label, value)
            }
            ComponentKind::Row {
                children,
                justify,
                align,
            } => self.stack(&ctx, children, Axis::Horizontal, *justify, *align),
            ComponentKind::Column {
                children,
                justify,
                align,
            } => self.stack(&ctx, children, Axis::Vertical, *justify, *align),
            ComponentKind::List {
                children,
                direction,
                align,
            } => self.list(&ctx, children, *direction, *align),
            ComponentKind::Card { child } => self.nested_card(&ctx, child),
            ComponentKind::Button {
                child,
                variant,
                action,
            } => self.button(&ctx, c, child, *variant, action),
            ComponentKind::Tabs { tabs } => self.tabs(&ctx, c, tabs),
            ComponentKind::Unknown { name, .. } => {
                self.fallback_box(&format!("{name} (id \"{id}\")"))
            }
        };
        match c.weight {
            Some(w) if w > 0.0 => div().flex_1().min_w_0().child(el).into_any_element(),
            _ => el,
        }
    }

    fn children(&self, ctx: &Ctx, list: &ChildList) -> Vec<AnyElement> {
        let is_template = matches!(list, ChildList::Template { .. });
        expand_children(self.card, self.state, &ctx.scope, list, MAX_TEMPLATE_ITEMS)
            .into_iter()
            .enumerate()
            .map(|(ix, (id, scope))| {
                if self.budget.exceeded() {
                    return gpui::Empty.into_any_element();
                }
                let sub = Ctx {
                    scope,
                    depth: ctx.depth,
                    ink: ctx.ink,
                    instance: if is_template {
                        format!("{}{ix}:", ctx.instance)
                    } else {
                        ctx.instance.clone()
                    },
                };
                self.component(&id, &sub)
            })
            .collect()
    }

    fn stack(
        &self,
        ctx: &Ctx,
        children: &ChildList,
        axis: Axis,
        justify: Justify,
        align: Align,
    ) -> AnyElement {
        let mut el = div().flex().w_full().min_w_0().gap(px(STACK_GAP));
        el = match axis {
            Axis::Horizontal => el.flex_row().flex_wrap(),
            Axis::Vertical => el.flex_col(),
        };
        el = match justify {
            Justify::Start | Justify::Stretch => el.justify_start(),
            Justify::Center => el.justify_center(),
            Justify::End => el.justify_end(),
            Justify::SpaceBetween => el.justify_between(),
            Justify::SpaceAround => el.justify_around(),
            Justify::SpaceEvenly => el.justify_evenly(),
        };
        el = match align {
            Align::Start => el.items_start(),
            Align::Center => el.items_center(),
            Align::End => el.items_end(),
            Align::Stretch => el.items_stretch(),
        };
        el.children(self.children(ctx, children)).into_any_element()
    }

    /// A List is a stack with hairlines between items (vertical) or a
    /// wrapping row (horizontal). Inner scrolling is a follow-up: the
    /// transcript row measures the whole card, so it grows instead.
    fn list(&self, ctx: &Ctx, children: &ChildList, direction: Axis, align: Align) -> AnyElement {
        let items = self.children(ctx, children);
        let theme = self.theme;
        let mut el = div().flex().w_full().min_w_0();
        el = match align {
            Align::Start => el.items_start(),
            Align::Center => el.items_center(),
            Align::End => el.items_end(),
            Align::Stretch => el.items_stretch(),
        };
        match direction {
            Axis::Horizontal => el
                .flex_row()
                .flex_wrap()
                .gap(px(STACK_GAP))
                .children(items)
                .into_any_element(),
            Axis::Vertical => {
                let last = items.len().saturating_sub(1);
                el.flex_col()
                    .children(items.into_iter().enumerate().map(|(ix, item)| {
                        div()
                            .w_full()
                            .min_w_0()
                            .py(px(6.0))
                            .when(ix < last, |d| d.border_b_1().border_color(theme.border))
                            .child(item)
                    }))
                    .into_any_element()
            }
        }
    }

    /// A Card inside the tree: transparent plate with a hairline, so nested
    /// cards stay distinct at any depth (the catalog guide's advice).
    fn nested_card(&self, ctx: &Ctx, child: &str) -> AnyElement {
        div()
            .w_full()
            .min_w_0()
            .rounded(px(self.theme.control_radius + 2.0))
            .border_1()
            .border_color(self.theme.border)
            .p(px(12.0))
            .child(self.component(child, ctx))
            .into_any_element()
    }

    fn button(
        &self,
        ctx: &Ctx,
        c: &Component,
        child: &str,
        variant: ButtonVariant,
        action: &Action,
    ) -> AnyElement {
        let theme = self.theme;
        let ink = match variant {
            ButtonVariant::Primary => theme.on_solid,
            ButtonVariant::Default => theme.text,
            ButtonVariant::Borderless => theme.text_muted,
        };
        let label_ctx = Ctx {
            ink: Some(ink),
            ..ctx.clone()
        };
        let label = self.component(child, &label_ctx);
        let event = self.state.resolve_action(self.card, &ctx.scope, action);
        let on_event = self.on_event.clone();
        let mut el = div()
            .id(self.eid(ctx, &c.id))
            .flex()
            .flex_none()
            .flex_row()
            .items_center()
            .justify_center()
            .gap(px(6.0))
            .h(px(30.0))
            .px(px(12.0))
            .rounded(px(theme.control_radius))
            .text_size(rems(13.0 / 16.0))
            .line_height(px(18.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(ink);
        el = match variant {
            ButtonVariant::Primary => el.bg(theme.solid).hover(|s| s.opacity(0.88)),
            ButtonVariant::Default => el
                .bg(theme.surface_raised)
                .border_1()
                .border_color(theme.border)
                .hover(|s| s.bg(theme.element_hover)),
            ButtonVariant::Borderless => el.hover(|s| s.bg(theme.element_hover).text_color(theme.text)),
        };
        el = match event {
            Some(event) => el
                .cursor_pointer()
                .on_click(move |_: &ClickEvent, window, cx| on_event(event.clone(), window, cx)),
            None => el.opacity(0.55),
        };
        el.child(label).into_any_element()
    }

    fn tabs(&self, ctx: &Ctx, c: &Component, tabs: &[Tab]) -> AnyElement {
        if tabs.is_empty() {
            return self.fallback_box(&format!("Tabs \"{}\" has no tabs", c.id));
        }
        let theme = self.theme;
        let selected = self.state.selected_tab(&c.id).min(tabs.len() - 1);
        let header = div()
            .flex()
            .flex_row()
            .flex_wrap()
            .gap(px(2.0))
            .w_full()
            .border_b_1()
            .border_color(theme.border)
            .children(tabs.iter().enumerate().map(|(ix, tab)| {
                let title = crate::leaf::clip(
                    &resolve_string(&self.state.data, &ctx.scope, &tab.title),
                    crate::leaf::MAX_LABEL_CHARS,
                );
                let active = ix == selected;
                let on_event = self.on_event.clone();
                let component_id = c.id.clone();
                div()
                    .id(self.eid(ctx, &format!("{}#tab{ix}", c.id)))
                    .px(px(10.0))
                    .py(px(6.0))
                    .mb(px(-1.0))
                    .cursor_pointer()
                    .text_size(rems(13.0 / 16.0))
                    .line_height(px(18.0))
                    .font_weight(if active {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if active { theme.text } else { theme.text_muted })
                    .border_b_1()
                    .border_color(if active {
                        theme.accent
                    } else {
                        gpui::transparent_black()
                    })
                    .hover(|s| s.text_color(theme.text))
                    .on_click(move |_: &ClickEvent, window, cx| {
                        on_event(
                            CardEvent::SelectTab {
                                component_id: component_id.clone(),
                                index: ix,
                            },
                            window,
                            cx,
                        )
                    })
                    .child(SharedString::from(title))
            }));
        div()
            .flex()
            .flex_col()
            .w_full()
            .min_w_0()
            .gap(px(10.0))
            .child(header)
            .child(self.component(&tabs[selected].child, ctx))
            .into_any_element()
    }

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
    fn diagnostics(&self, whole: bool, headline: Option<String>) -> AnyElement {
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
