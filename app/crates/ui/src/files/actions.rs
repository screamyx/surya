//! File action forms and their engine round trip.

use super::*;
use crate::composer::{ComposerInput, ComposerInputEvent};
use crate::popover;
use gpui::{AnyElement, Focusable as _, Task};
use surya_proto::files::FileMutation;
use surya_rpc::methods;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileAction {
    Create(String),
    Rename(String),
    Delete(String),
}

impl FileAction {
    fn title(&self) -> &'static str {
        match self {
            Self::Create(_) => "New file",
            Self::Rename(_) => "Rename file",
            Self::Delete(_) => "Delete file",
        }
    }

    fn path(&self) -> &str {
        match self {
            Self::Create(path) | Self::Rename(path) | Self::Delete(path) => path,
        }
    }

    fn mutation(&self, name: &str) -> Result<FileMutation, &'static str> {
        if let Self::Delete(path) = self {
            return Ok(FileMutation::Delete { path: path.clone() });
        }
        let name = name.trim();
        if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\', ':']) {
            return Err("Enter a file name without folders.");
        }
        let dir = match self {
            Self::Create(dir) => dir.as_str(),
            Self::Rename(path) => parent(path),
            Self::Delete(_) => unreachable!(),
        };
        let target = if dir.is_empty() {
            name.into()
        } else {
            format!("{dir}/{name}")
        };
        Ok(match self {
            Self::Create(_) => FileMutation::Create { path: target },
            Self::Rename(path) => FileMutation::Rename {
                path: path.clone(),
                new_path: target,
            },
            Self::Delete(_) => unreachable!(),
        })
    }
}

pub(super) fn parent(path: &str) -> &str {
    path.rsplit_once('/')
        .map(|(parent, _)| parent)
        .unwrap_or("")
}

pub(super) struct ActionForm {
    action: FileAction,
    name: Entity<ComposerInput>,
    focus_pending: bool,
    failure: Option<String>,
    busy: bool,
    _events: Subscription,
    task: Option<Task<()>>,
}

impl FilesPane {
    pub(super) fn begin_action(&mut self, action: FileAction, cx: &mut Context<Self>) {
        let name = cx.new(|cx| ComposerInput::new("File name", cx));
        if let FileAction::Rename(path) = &action {
            name.update(cx, |input, cx| {
                input.set_text(path.rsplit('/').next().unwrap_or(path), cx);
                input.set_select_all_on_focus(true);
            });
        }
        let events = cx.subscribe(&name, |this: &mut Self, _, event, cx| {
            if matches!(event, ComposerInputEvent::Submitted) {
                this.submit_action(cx);
            }
        });
        self.action_form = Some(ActionForm {
            action,
            name,
            focus_pending: true,
            failure: None,
            busy: false,
            _events: events,
            task: None,
        });
        cx.notify();
    }

    pub(super) fn submit_action(&mut self, cx: &mut Context<Self>) {
        let Some(form) = self.action_form.as_mut().filter(|form| !form.busy) else {
            return;
        };
        // The form replaces the editor while open. Refuse before dispatch
        // when it still holds edits or a save; nothing is silently discarded.
        if !matches!(form.action, FileAction::Create(_))
            && (self.editor.read(cx).is_dirty(cx) || self.editor.read(cx).doc.saving)
        {
            form.failure = Some("Save or discard editor changes before changing this file.".into());
            cx.notify();
            return;
        }
        let mutation = match form.action.mutation(form.name.read(cx).text()) {
            Ok(mutation) => mutation,
            Err(error) => {
                form.failure = Some(error.into());
                cx.notify();
                return;
            }
        };
        form.busy = true;
        form.failure = None;
        let engine = self.engine.clone();
        let params = serde_json::json!({ "spaceId": self.space_id, "action": mutation });
        form.task = Some(cx.spawn(async move |this, cx| {
            let result = engine.client().call(methods::FILES_MUTATE, params).await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(_) => {
                        this.tree.update(cx, |tree, cx| {
                            let (old, new) = match &mutation {
                                FileMutation::Create { path } => (None, Some(path)),
                                FileMutation::Rename { path, new_path } => {
                                    (Some(path), Some(new_path))
                                }
                                FileMutation::Delete { path } => (Some(path), None),
                            };
                            if let Some(path) = old {
                                tree.load(parent(path), cx);
                            }
                            if let Some(path) = new {
                                tree.load(parent(path), cx);
                            }
                            tree.model.selected = new.cloned();
                        });
                        this.editor
                            .update(cx, |editor, cx| editor.file_action_completed(&mutation, cx));
                        this.action_form = None;
                    }
                    Err(error) => {
                        if let Some(form) = &mut this.action_form {
                            form.busy = false;
                            form.failure = Some(error.to_string());
                        }
                    }
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    pub(super) fn render_action(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        // Focus first: `Theme::of` borrows `cx` for the rest of the render, and
        // `window.focus` needs it mutably.
        let action_focus = self.action_focus.clone();
        let form = self.action_form.as_mut()?;
        let deleting = matches!(form.action, FileAction::Delete(_));
        if form.focus_pending {
            if deleting {
                window.focus(&action_focus, cx);
            } else {
                let handle = form.name.read(cx).focus_handle(cx);
                window.focus(&handle, cx);
            }
            form.focus_pending = false;
        }
        let theme = Theme::of(cx);
        let form = self.action_form.as_ref()?;
        let busy = form.busy;
        let title = form.action.title();
        let path = form.action.path().to_string();
        let detail = if deleting {
            format!("Delete {path}? This removes the file from disk.")
        } else if path.is_empty() {
            "In the project root".into()
        } else {
            path
        };
        let submit = if deleting {
            popover::btn_danger(theme, "Delete")
        } else {
            popover::btn_primary(
                theme,
                if matches!(form.action, FileAction::Create(_)) {
                    "Create"
                } else {
                    "Rename"
                },
            )
        };
        Some(
            div()
                .id("file-action-form")
                .track_focus(&self.action_focus)
                .size_full()
                .p(px(16.0))
                .flex()
                .flex_col()
                .gap(px(12.0))
                .bg(theme.bg)
                .text_color(theme.text)
                .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                    if event.keystroke.key == "escape"
                        && this.action_form.as_ref().is_some_and(|form| !form.busy)
                    {
                        this.action_form = None;
                        cx.notify();
                    }
                }))
                .child(
                    div()
                        .text_size(px(14.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(title),
                )
                .child(div().text_size(px(12.0)).child(detail))
                .when(!deleting, |el| {
                    el.child(
                        div()
                            .p(px(8.0))
                            .rounded(px(6.0))
                            .bg(theme.input_bg)
                            .child(form.name.clone()),
                    )
                })
                .when_some(form.failure.clone(), |el, error| {
                    el.child(div().text_color(theme.danger).child(error))
                })
                .when(busy, |el| el.child("Updating file…"))
                .when(!busy, |el| {
                    el.child(
                        div()
                            .flex()
                            .gap(px(8.0))
                            .child(
                                submit
                                    .id("file-action-submit")
                                    .on_click(cx.listener(|this, _, _, cx| this.submit_action(cx))),
                            )
                            .child(
                                popover::btn_ghost(theme, "Cancel", "file-action-cancel")
                                    .id("file-action-cancel")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.action_form = None;
                                        cx.notify();
                                    })),
                            ),
                    )
                })
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actions_keep_the_selected_folder_and_reject_path_injection() {
        assert_eq!(
            FileAction::Create("src".into())
                .mutation("notes.txt")
                .unwrap(),
            FileMutation::Create {
                path: "src/notes.txt".into()
            }
        );
        assert_eq!(
            FileAction::Rename("src/old.txt".into())
                .mutation("new.txt")
                .unwrap(),
            FileMutation::Rename {
                path: "src/old.txt".into(),
                new_path: "src/new.txt".into()
            }
        );
        for name in ["", ".", "..", "../other", "C:\\other", "folder/file"] {
            assert!(FileAction::Create("".into()).mutation(name).is_err());
        }
        assert_eq!(
            FileAction::Delete("src/notes.txt".into())
                .mutation("")
                .unwrap(),
            FileMutation::Delete {
                path: "src/notes.txt".into()
            }
        );
    }
}
