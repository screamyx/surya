//! One task card: title, running bar, notes preview, owner and link chips,
//! footer; the gpui drag source and drop target for the board.

use gpui::{AnyElement, Context, IntoElement, Render, SharedString, Window, div, prelude::*, px};

use zeron_proto::{Task as BoardTask, TaskStatus};

use super::board::TasksPane;
use super::chips::{link_chip, owner_chip, preview, short_id};
use super::model::DropTarget;
use crate::theme::{Theme, hairline};
use crate::typography::ui_rems;

/// gpui drag payload: which card is in the air.
pub struct CardDrag {
    pub task_id: String,
    pub title: SharedString,
}

/// The floating copy under the pointer while dragging.
struct CardGhost {
    title: SharedString,
}

impl Render for CardGhost {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx);
        div()
            .w(px(240.0))
            .px(px(12.0))
            .py(px(10.0))
            .rounded(px(12.0))
            .bg(theme.surface_card)
            .border_1()
            .border_color(theme.border_strong)
            .shadow_lg()
            .text_size(ui_rems(13.0))
            .text_color(theme.text)
            .child(self.title.clone())
    }
}

impl TasksPane {
    pub(super) fn render_card(
        &self,
        key: usize,
        task: &BoardTask,
        selected: bool,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = task.id.clone();
        let drop_id = task.id.clone();
        let select_id = task.id.clone();
        let title: SharedString = task.title.clone().into();
        let ghost_title = title.clone();
        let accent = theme.accent;
        let added: SharedString = format!(
            "{} · added {}",
            short_id(&task.id),
            task.created_at
                .with_timezone(&chrono::Local)
                .format("%H:%M")
        )
        .into();
        let mut chips: Vec<AnyElement> = vec![owner_chip(theme, task.owner.as_deref())];
        for link in &task.links {
            chips.push(link_chip(theme, link));
        }
        let notes = task
            .notes
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(|n| SharedString::from(preview(n, 90)));
        div()
            .id(("task-card", key))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .px(px(12.0))
            .py(px(10.0))
            .rounded(px(12.0))
            .bg(theme.surface_card) // TOKEN: surya.card_bg
            .border_1()
            .border_color(if selected { accent } else { hairline(0.08) })
            .shadow_sm()
            .cursor_grab()
            .hover(|s| s.bg(theme.surface_raised_hover))
            .drag_over::<CardDrag>(move |style, _, _, _| style.border_color(accent))
            .on_drag(
                CardDrag {
                    task_id: id,
                    title: ghost_title,
                },
                |drag, _point, _, cx| {
                    let title = drag.title.clone();
                    cx.stop_propagation();
                    cx.new(|_| CardGhost { title })
                },
            )
            .on_drop::<CardDrag>(cx.listener(move |this, drag: &CardDrag, _, cx| {
                cx.stop_propagation();
                this.drop_card(
                    drag.task_id.clone(),
                    DropTarget::Before(drop_id.clone()),
                    cx,
                );
            }))
            .on_click(cx.listener(move |this, event: &gpui::ClickEvent, _, cx| {
                this.model.select(Some(select_id.clone()));
                if event.click_count() >= 2
                    && let Some(task) = this.model.task(&select_id).cloned()
                {
                    this.open_sheet(Some(&task), cx);
                }
                cx.notify();
            }))
            .child(
                div()
                    .text_size(ui_rems(13.5))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(theme.text)
                    .child(title),
            )
            .when(task.status == TaskStatus::Running, |el| {
                el.child(
                    div()
                        .h(px(3.0))
                        .rounded(px(2.0))
                        .bg(theme.accent) // TOKEN: surya.progress
                        .w(gpui::relative(0.6)),
                )
            })
            .children(notes.map(|n| {
                div()
                    .text_size(ui_rems(12.0))
                    .text_color(theme.text_muted)
                    .child(n)
            }))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap(px(6.0))
                    .children(chips),
            )
            .child(
                div()
                    .text_size(ui_rems(11.5))
                    .text_color(theme.text_faint)
                    .child(added),
            )
            .into_any_element()
    }
}
