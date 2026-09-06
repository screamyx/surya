//! The edit sheet: title, notes, owner, status, links; delete with a confirm
//! step. One modal card over the board (`popover::modal`), inputs are
//! `ComposerInput` entities, submit = Enter in title/owner or the button.

use gpui::{
    AnyElement, Entity, Focusable as _, Pixels, SharedString, Size, Subscription, Window, div,
    prelude::*, px,
};

use surya_proto::{Task, TaskStatus};

use super::board::TasksPane;
use super::model::{COLUMNS, column_label};
use crate::composer::{ComposerInput, ComposerInputEvent};
use crate::popover;
use crate::theme::{Theme, hairline, ink};
use crate::typography::ui_rems;

pub struct EditSheet {
    /// `None` = creating a new task.
    task_id: Option<String>,
    title: Entity<ComposerInput>,
    notes: Entity<ComposerInput>,
    owner: Entity<ComposerInput>,
    links: Entity<ComposerInput>,
    status: TaskStatus,
    confirm_delete: bool,
    focus_pending: bool,
    _events: Vec<Subscription>,
}

/// Links are typed one per line or space-separated; keep the URLs.
pub fn parse_links(text: &str) -> Vec<String> {
    text.split_whitespace()
        .filter(|w| w.starts_with("http://") || w.starts_with("https://"))
        .map(str::to_string)
        .collect()
}

/// Owner text → row value: empty / "you" / "user" = unowned (`null`).
pub fn parse_owner(text: &str) -> Option<String> {
    let owner = text.trim();
    if owner.is_empty() || owner.eq_ignore_ascii_case("you") || owner.eq_ignore_ascii_case("user") {
        None
    } else {
        Some(owner.to_string())
    }
}

impl TasksPane {
    pub(super) fn open_sheet(&mut self, task: Option<&Task>, cx: &mut gpui::Context<Self>) {
        let title = cx.new(|cx| ComposerInput::new("Title", cx));
        let notes = cx.new(|cx| ComposerInput::new("Notes (shift-enter for a new line)", cx));
        let owner = cx.new(|cx| ComposerInput::new("Owner: agent id, or leave empty for you", cx));
        let links = cx.new(|cx| ComposerInput::new("Links: one URL per line", cx));
        if let Some(task) = task {
            title.update(cx, |i, cx| i.set_text(task.title.clone(), cx));
            notes.update(cx, |i, cx| {
                i.set_text(task.notes.clone().unwrap_or_default(), cx)
            });
            owner.update(cx, |i, cx| {
                i.set_text(task.owner.clone().unwrap_or_default(), cx)
            });
            links.update(cx, |i, cx| i.set_text(task.links.join("\n"), cx));
        }
        let submit = |this: &mut Self,
                      _: Entity<ComposerInput>,
                      event: &ComposerInputEvent,
                      cx: &mut gpui::Context<Self>| {
            if matches!(event, ComposerInputEvent::Submitted) {
                this.submit_sheet(cx);
            }
        };
        let events = vec![cx.subscribe(&title, submit), cx.subscribe(&owner, submit)];
        self.sheet = Some(EditSheet {
            task_id: task.map(|t| t.id.clone()),
            title,
            notes,
            owner,
            links,
            status: task.map_or(TaskStatus::Queued, |t| t.status),
            confirm_delete: false,
            focus_pending: true,
            _events: events,
        });
        cx.notify();
    }

    fn submit_sheet(&mut self, cx: &mut gpui::Context<Self>) {
        let Some(sheet) = self.sheet.take() else {
            return;
        };
        let title = sheet.title.read(cx).text().trim().to_string();
        if title.is_empty() {
            // Keep the sheet open: an empty title is not a task.
            self.sheet = Some(sheet);
            cx.notify();
            return;
        }
        let notes = sheet.notes.read(cx).text().trim().to_string();
        let owner = parse_owner(sheet.owner.read(cx).text());
        let links = parse_links(sheet.links.read(cx).text());
        match sheet.task_id {
            None => {
                let extra = serde_json::json!({
                    "notes": (!notes.is_empty()).then_some(notes),
                    "owner": owner,
                    "links": links,
                });
                self.create_task(&title, sheet.status, extra, cx);
            }
            Some(task_id) => {
                self.mutate(
                    serde_json::json!({
                        "op": "updateTask",
                        "taskId": task_id,
                        "title": title,
                        "status": sheet.status,
                        "notes": (!notes.is_empty()).then_some(notes),
                        "owner": owner,
                        "links": links,
                    }),
                    cx,
                );
            }
        }
        cx.notify();
    }

    fn delete_from_sheet(&mut self, cx: &mut gpui::Context<Self>) {
        let Some(sheet) = self.sheet.take() else {
            return;
        };
        if let Some(task_id) = sheet.task_id {
            self.mutate(
                serde_json::json!({ "op": "deleteTask", "taskId": task_id }),
                cx,
            );
        }
        cx.notify();
    }

    pub(super) fn render_sheet(
        &mut self,
        viewport: Size<Pixels>,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) -> Option<AnyElement> {
        let theme = Theme::of(cx).clone();
        let sheet = self.sheet.as_mut()?;
        if std::mem::take(&mut sheet.focus_pending) {
            window.focus(&sheet.title.focus_handle(cx), cx);
        }
        let editing = sheet.task_id.is_some();
        let status = sheet.status;
        let confirm_delete = sheet.confirm_delete;
        let (title, notes, owner, links) = (
            sheet.title.clone(),
            sheet.notes.clone(),
            sheet.owner.clone(),
            sheet.links.clone(),
        );
        let status_chips: Vec<AnyElement> = COLUMNS
            .iter()
            .enumerate()
            .map(|(ix, s)| {
                let active = *s == status;
                let pick = *s;
                div()
                    .id(("task-sheet-status", ix))
                    .px(px(10.0))
                    .py(px(4.0))
                    .rounded(px(999.0))
                    .border_1()
                    .border_color(if active { theme.accent } else { hairline(0.10) })
                    .bg(if active { theme.accent_wash } else { ink(0.03) }) // TOKEN: surya.chip_active
                    .text_size(ui_rems(12.0))
                    .text_color(if active { theme.text } else { theme.text_muted })
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(sheet) = &mut this.sheet {
                            sheet.status = pick;
                        }
                        cx.notify();
                    }))
                    .child(column_label(*s))
                    .into_any_element()
            })
            .collect();
        let field = |label: &str, input: Entity<ComposerInput>| {
            div()
                .mt(px(10.0))
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(
                    div()
                        .text_size(ui_rems(11.5))
                        .text_color(theme.text_faint)
                        .child(SharedString::from(label.to_string())),
                )
                .child(popover::dialog_field(input.into_any_element()))
        };
        let footer = if confirm_delete {
            div()
                .mt(px(16.0))
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .child(popover::dialog_body(
                    &theme,
                    "Delete this task? Agents lose it too.",
                ))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(8.0))
                        .child(
                            popover::btn_ghost(&theme, "Keep", "task-sheet-keep")
                                .id("task-sheet-keep")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(sheet) = &mut this.sheet {
                                        sheet.confirm_delete = false;
                                    }
                                    cx.notify();
                                })),
                        )
                        .child(
                            popover::btn_danger(&theme, "Delete")
                                .id("task-sheet-delete-confirm")
                                .on_click(cx.listener(|this, _, _, cx| this.delete_from_sheet(cx))),
                        ),
                )
        } else {
            div()
                .mt(px(16.0))
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .child(div().children(editing.then(|| {
                    popover::btn_ghost(&theme, "Delete…", "task-sheet-delete")
                        .id("task-sheet-delete")
                        .text_color(theme.danger)
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(sheet) = &mut this.sheet {
                                sheet.confirm_delete = true;
                            }
                            cx.notify();
                        }))
                })))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(8.0))
                        .child(
                            popover::btn_ghost(&theme, "Cancel", "task-sheet-cancel")
                                .id("task-sheet-cancel")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.sheet = None;
                                    cx.notify();
                                })),
                        )
                        .child(
                            popover::btn_primary(&theme, if editing { "Save" } else { "Add task" })
                                .id("task-sheet-save")
                                .on_click(cx.listener(|this, _, _, cx| this.submit_sheet(cx))),
                        ),
                )
        };
        let card = popover::dialog_card(&theme)
            .w(px(460.0))
            .on_key_down(cx.listener(|this, ev: &gpui::KeyDownEvent, _, cx| {
                if ev.keystroke.key == "escape" {
                    this.sheet = None;
                    cx.notify();
                }
            }))
            .child(popover::dialog_title(
                &theme,
                if editing { "Edit task" } else { "New task" },
            ))
            .child(field("Title", title))
            .child(
                div()
                    .mt(px(10.0))
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(
                        div()
                            .text_size(ui_rems(11.5))
                            .text_color(theme.text_faint)
                            .child("Status"),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .flex_wrap()
                            .gap(px(6.0))
                            .children(status_chips),
                    ),
            )
            .child(field("Owner", owner))
            .child(field("Notes", notes))
            .child(field("Links", links))
            .child(footer)
            .into_any_element();
        Some(popover::modal("task-edit-sheet", viewport, card))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_and_owner_parse() {
        assert_eq!(
            parse_links("https://a.com/x\nnot a link http://b.org/y"),
            vec!["https://a.com/x".to_string(), "http://b.org/y".to_string()]
        );
        assert_eq!(parse_owner(""), None);
        assert_eq!(parse_owner(" You "), None);
        assert_eq!(parse_owner("user"), None);
        assert_eq!(parse_owner("raven"), Some("raven".to_string()));
    }
}
