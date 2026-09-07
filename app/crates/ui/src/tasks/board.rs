//! The Tasks pane: four columns of cards live from `WatchTasks`, drag between
//! columns (status) and within one (rank), keyboard up/down/enter/n, a quick
//! add box on Queued. Layout follows `mockup/shots/tasks-desktop.png`.
//!
//! Colours are the stock theme; `// TOKEN:` marks where surya-theme's tokens
//! (`crates/ui/src/surya.rs` on feat/surya-look) go.

use std::sync::Arc;

use gpui::{
    AnyElement, App, Context, Entity, FocusHandle, Focusable, IntoElement, KeyDownEvent, Render,
    SharedString, Subscription, Task, Window, div, prelude::*, px,
};

use surya_proto::{Task as BoardTask, TaskStatus};
use surya_rpc::{RpcClient, methods};

use super::card::CardDrag;
use super::chips::count_chip;
use super::edit::EditSheet;
use super::columns_strip;
use super::header;
use super::model::{BoardModel, COLUMNS, DropTarget, column_label};
use crate::composer::{ComposerInput, ComposerInputEvent};
use crate::theme::{Theme, hairline};
use crate::typography::ui_rems;

/// A card column never squeezes below this; the row scrolls instead.
pub(super) const COLUMN_MIN_W: f32 = 220.0;

/// The gap between columns. Named because the fade maths and the column
/// strip both have to agree with what the row actually lays out.
pub(super) const COLUMN_GAP: f32 = 16.0;

pub struct TasksPane {
    client: Arc<RpcClient>,
    space_id: String,
    space_name: SharedString,
    pub(super) model: BoardModel,
    pub(super) focus_handle: FocusHandle,
    pub(super) sheet: Option<EditSheet>,
    pub(super) quick_add: Entity<ComposerInput>,
    /// Last RPC failure, shown under the header until the next success.
    pub(super) error: Option<SharedString>,
    /// The columns row scrolls sideways when the pane is narrower than four
    /// columns; the offset drives the edge fades that say more is hiding.
    columns_scroll: gpui::ScrollHandle,
    _watch: Task<()>,
    _quick_add_events: Subscription,
}

/// The error banner's painted pair: the ink, and the plane it sits on.
///
/// One named place on purpose. The banner used to put `danger` text on
/// `danger_muted`, which reads like the obvious pairing and is not:
/// `danger_muted` is opaque and close enough to `danger` in luminance that
/// the pair measures 1.51 dark and 1.35 light, against the 4.5 text minimum.
/// On the board's own plane it measures 7.01 and 4.76.
///
/// A danger tint was measured too, at 0.08, 0.12 and 0.16 alpha. Every
/// strength failed in light, because light only has 4.76 to spend before any
/// tint eats into it. So the banner keeps its shape with a danger hairline
/// instead of a fill; a border is not text and carries no contrast floor.
///
/// Returning the pair rather than building the element keeps it measurable:
/// the test at the bottom of this file asserts THIS function, so a future
/// change to the pair fails there rather than only in a screenshot.
pub(crate) fn error_banner_colors(theme: &Theme) -> (gpui::Hsla, gpui::Hsla) {
    (theme.danger, theme.bg)
}

impl TasksPane {
    /// `space` is the board (a space id); `space_name` is what the header shows.
    pub fn new(
        client: Arc<RpcClient>,
        space: impl Into<String>,
        space_name: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let space_id: String = space.into();
        let quick_add = cx.new(|cx| ComposerInput::with_context("Add a task", "PaletteSearch", cx));
        // Kept, but it does not fire today: `PaletteSearch` leaves enter
        // unbound, so the input never raises `Submitted` and `quick_add.rs`
        // handles the key instead. This stays correct if that binding lands.
        let quick_add_events = cx.subscribe(&quick_add, |this: &mut Self, _, event, cx| {
            if matches!(event, ComposerInputEvent::Submitted) {
                this.submit_quick_add(cx);
            }
        });
        let watch = spawn_watch(client.clone(), space_id.clone(), cx);
        Self {
            client,
            space_id,
            space_name: space_name.into(),
            model: BoardModel::default(),
            focus_handle: cx.focus_handle(),
            columns_scroll: gpui::ScrollHandle::new(),
            sheet: None,
            quick_add,
            // Normally None. `SURYA_DEMO_TASK_ERROR` paints the banner on a
            // board that no engine failure could ever show; see
            // `demo_banner`.
            error: super::demo_banner::from_env().map(SharedString::from),
            _watch: watch,
            _quick_add_events: quick_add_events,
        }
    }

    pub fn space_id(&self) -> &str {
        &self.space_id
    }

    /// Fire one `Mutate`; failures land in `error`, successes clear it.
    pub(super) fn mutate(&mut self, params: serde_json::Value, cx: &mut Context<Self>) {
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = client.call(methods::MUTATE, params).await;
            let _ = this.update(cx, |pane, cx| {
                if let Err(err) = result {
                    pane.error = Some(SharedString::from(err.to_string()));
                    pane.model.clear_pending();
                } else {
                    pane.error = None;
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn create_task(
        &mut self,
        title: &str,
        status: TaskStatus,
        extra: serde_json::Value,
        cx: &mut Context<Self>,
    ) {
        let title = title.trim();
        if title.is_empty() {
            return;
        }
        let mut params = serde_json::json!({
            "op": "createTask",
            "taskId": uuid::Uuid::new_v4().to_string(),
            "spaceId": self.space_id,
            "title": title,
            "status": status,
            "rank": self.model.append_rank(status),
        });
        if let (Some(out), Some(more)) = (params.as_object_mut(), extra.as_object()) {
            for (k, v) in more {
                if !v.is_null() {
                    out.insert(k.clone(), v.clone());
                }
            }
        }
        self.mutate(params, cx);
    }

    /// A card was dropped: status change and/or rank writes per the model.
    pub(super) fn drop_card(
        &mut self,
        task_id: String,
        target: DropTarget,
        cx: &mut Context<Self>,
    ) {
        let Some(plan) = self.model.plan_drop(&task_id, &target) else {
            return;
        };
        // Move the card now; the watch frames reconcile (model.rs).
        self.model
            .apply_optimistic(plan.clone(), std::time::Instant::now());
        if let Some(status) = plan.status {
            self.mutate(
                serde_json::json!({ "op": "updateTask", "taskId": plan.task_id, "status": status }),
                cx,
            );
        }
        for (id, rank) in plan.ranks {
            self.mutate(
                serde_json::json!({ "op": "reorderTask", "taskId": id, "rank": rank }),
                cx,
            );
        }
        self.model.select(Some(task_id));
        cx.notify();
    }

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        // The quick-add box first: enter and escape are unbound in its key
        // context on purpose, so they arrive here as raw keys while the box,
        // not the pane, holds focus. See `quick_add.rs`.
        if self.on_quick_add_key(event, window, cx) {
            cx.stop_propagation();
            return;
        }
        // Only when the pane itself has focus: a child input owns its own keys.
        if !self.focus_handle.is_focused(window) {
            return;
        }
        match event.keystroke.key.as_str() {
            "up" => self.model.step(-1),
            "down" => self.model.step(1),
            "enter" => {
                if let Some(task) = self
                    .model
                    .selected()
                    .and_then(|id| self.model.task(id))
                    .cloned()
                {
                    self.open_sheet(Some(&task), cx);
                }
            }
            "n" => self.open_sheet(None, cx),
            "escape" => self.sheet = None,
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn render_column(
        &mut self,
        ix: usize,
        status: TaskStatus,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let tasks: Vec<BoardTask> = self.model.column(status).into_iter().cloned().collect();
        let selected = self.model.selected().map(str::to_string);
        let accent = theme.accent;
        let cards: Vec<AnyElement> = tasks
            .iter()
            .enumerate()
            .map(|(cix, task)| {
                let is_selected = selected.as_deref() == Some(task.id.as_str());
                self.render_card(ix * 1000 + cix, task, is_selected, theme, cx)
            })
            .collect();
        let quick_add = (status == TaskStatus::Queued).then(|| {
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .mb(px(4.0))
                .child(
                    div()
                        .flex_1()
                        // The input reports a wide minimum; without this the
                        // row overflows and pushes the button out of the column.
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .px(px(10.0))
                        .py(px(6.0))
                        .rounded(px(8.0))
                        .bg(theme.input_bg) // TOKEN: surya.input_bg
                        .border_1()
                        .border_color(hairline(0.08))
                        .text_size(ui_rems(13.0))
                        .child(self.quick_add.clone()),
                )
                .child(
                    div()
                        .id("task-quick-add-go")
                        .flex_none()
                        .w(px(28.0))
                        .h(px(28.0))
                        .rounded(px(8.0))
                        .bg(theme.accent) // TOKEN: surya.accent
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .hover(|s| s.opacity(0.9))
                        .on_click(cx.listener(|this, _, _, cx| this.submit_quick_add(cx)))
                        .child(
                            crate::icons::icon(crate::icons::PLUS)
                                .size(px(14.0))
                                .text_color(theme.on_accent),
                        ),
                )
        });
        div()
            .id(("task-column", ix))
            .flex_1()
            .min_w(px(COLUMN_MIN_W))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(6.0))
                    .px(px(2.0))
                    .child(
                        div()
                            .text_size(ui_rems(13.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.text)
                            .child(column_label(status)),
                    )
                    .child(count_chip(theme, tasks.len())),
            )
            .child(
                div()
                    .id(("task-column-body", ix))
                    .flex_1()
                    .min_h(px(160.0))
                    .p(px(8.0))
                    .rounded(px(Theme::PANEL_RADIUS))
                    .bg(theme.surface) // TOKEN: surya.column_bg
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .overflow_y_scroll()
                    .drag_over::<CardDrag>(move |style, _, _, _| {
                        style.border_1().border_color(accent)
                    })
                    .on_drop::<CardDrag>(cx.listener(move |this, drag: &CardDrag, _, cx| {
                        this.drop_card(drag.task_id.clone(), DropTarget::End(status), cx);
                    }))
                    .children(quick_add)
                    .children(cards),
            )
    }
}

impl Focusable for TasksPane {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TasksPane {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let columns: Vec<_> = COLUMNS
            .iter()
            .enumerate()
            .map(|(ix, status)| self.render_column(ix, *status, &theme, cx))
            .collect();
        // The pair comes from one named place so the contrast test can
        // measure what actually paints (E2E-192).
        let (banner_ink, banner_plane) = error_banner_colors(&theme);
        let error = self.error.clone().map(|message| {
            div()
                .mx(px(16.0))
                .mb(px(8.0))
                .px(px(10.0))
                .py(px(6.0))
                .rounded(px(8.0))
                .bg(banner_plane)
                .border_1()
                .border_color(theme.danger)
                .text_size(ui_rems(12.5))
                .text_color(banner_ink)
                .child(message)
        });
        let sheet = self.render_sheet(window.viewport_size(), window, cx);
        let board = columns_strip::render_scroller(&self.columns_scroll, columns, &theme);
        div()
            .id("tasks-pane")
            .track_focus(&self.focus_handle)
            .on_key_down(
                cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    this.on_key(event, window, cx)
                }),
            )
            .on_click(cx.listener(|this, _, window, cx| {
                // Only when focus is not already inside the pane. A click on
                // the quick-add input focuses the input on the press and then
                // bubbles up to here, so an unconditional focus took it
                // straight back and the box never got a caret.
                if !this.focus_handle.contains_focused(window, cx) {
                    window.focus(&this.focus_handle, cx);
                }
            }))
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.bg) // TOKEN: surya.page_bg
            .text_color(theme.text)
            .child(header::render(&self.space_name, self.model.len(), &theme))
            .child(columns_strip::render(
                &columns_strip::entries(
                    |status| self.model.column(status).len(),
                    -f32::from(self.columns_scroll.offset().x),
                    f32::from(self.columns_scroll.bounds().size.width),
                    COLUMN_MIN_W,
                    COLUMN_GAP,
                ),
                &theme,
            ))
            .children(error)
            .child(board)
            .children(sheet)
    }
}

fn spawn_watch(client: Arc<RpcClient>, space_id: String, cx: &mut Context<TasksPane>) -> Task<()> {
    cx.spawn(async move |this, cx| {
        const RETRY: std::time::Duration = std::time::Duration::from_secs(2);
        loop {
            let params = serde_json::json!({ "spaceId": space_id });
            let mut rx = match client.subscribe(methods::WATCH_TASKS, params).await {
                Ok(rx) => rx,
                Err(err) => {
                    let alive = this.update(cx, |pane, cx| {
                        pane.error = Some(format!("task board unavailable: {err}").into());
                        cx.notify();
                    });
                    if alive.is_err() {
                        return;
                    }
                    cx.background_executor().timer(RETRY).await;
                    continue;
                }
            };
            while let Some(value) = rx.recv().await {
                let tasks: Vec<BoardTask> = match serde_json::from_value(value) {
                    Ok(tasks) => tasks,
                    Err(err) => {
                        tracing::warn!(error = %err, "dropping malformed WatchTasks frame");
                        continue;
                    }
                };
                if this
                    .update(cx, |pane, cx| {
                        pane.model.apply(tasks);
                        cx.notify();
                    })
                    .is_err()
                {
                    return;
                }
            }
            if this.update(cx, |_, _| {}).is_err() {
                return;
            }
            cx.background_executor().timer(RETRY).await;
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Tasks board's error banner, measured as it actually paints.
    ///
    /// It used to be danger text on `danger_muted`, which reads like an
    /// obvious pairing and is not: `danger_muted` is opaque and close enough
    /// to `danger` in luminance that the pair measured 1.51 dark and 1.35
    /// light. The banner now puts danger text on the board's own plane and
    /// carries a danger hairline instead.
    ///
    /// This measures the SHIPPED pair rather than asserting the tokens
    /// separately, because separate tokens both look fine and the defect was
    /// only ever in their combination.
    #[test]
    fn the_task_board_error_banner_is_readable_in_both_appearances() {
        for (name, t) in [("dark", crate::theme::Theme::dark()), ("light", crate::theme::Theme::light())] {
            // The pair the banner ACTUALLY paints, from the one place that
            // owns it. Asserting `danger` on `bg` here instead would pass no
            // matter what the banner did, which is how a 1.40:1 pair lived
            // in the board for as long as it did.
            let (ink, plane) = error_banner_colors(&t);
            let painted = crate::theme::contrast_ratio(crate::theme::flatten(ink, plane), plane);
            assert!(
                painted >= 4.5,
                "{name} task-board error banner is {painted:.2}:1, below the 4.5 text minimum"
            );
        }
    }
}
