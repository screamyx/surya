//! The file editor: haktui's editor pane (plain text, save on Cmd/Ctrl-S,
//! conflict banner with reload / overwrite) on top of `FilesRead` and
//! `FilesWrite`. The buffer is comet's [`ComposerInput`] under its own key
//! context (`FilesEditor`, bound in `files::init`) so Enter inserts a line
//! instead of submitting. Save is the [`SaveFile`] action, bound in that
//! context by `files::bind_save_keys` and handled here, so the shell's
//! keymap never sees the chord while the buffer has focus: a raw key
//! listener ran after the keymap and never got it (e2e FILES-01).
//!
//! [`EditorDoc`] holds every rule that is not drawing and is unit-tested:
//! what a read becomes, when a save may go, what a refusal does.

use gpui::{
    Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement, Render, Subscription,
    Task, Window, div, prelude::*, px,
};
use zeron_proto::files::{FileRead, FileWrite, LineRange};
use zeron_rpc::methods;

pub use super::editor_doc::{Body, Conflict, EditorDoc, SaveOutcome, Switch};
use super::SaveFile;
use crate::composer::{ComposerInput, ComposerInputEvent};
use crate::state::EngineHandle;
use crate::theme::Theme;

pub enum EditorEvent {
    Saved(String),
}

pub struct FileEditor {
    engine: EngineHandle,
    space_id: String,
    input: Entity<ComposerInput>,
    pub doc: EditorDoc,
    focus: FocusHandle,
    task: Option<Task<()>>,
    _input_events: Subscription,
}

impl EventEmitter<EditorEvent> for FileEditor {}

impl FileEditor {
    /// Buffer differs from the last read or save: closing would lose edits.
    pub fn is_dirty(&self, cx: &gpui::App) -> bool {
        self.doc.is_dirty(self.input.read(cx).text())
    }

    pub fn new(engine: EngineHandle, space_id: String, cx: &mut Context<Self>) -> Self {
        let input = cx
            .new(|cx| ComposerInput::with_context("Open a file from the tree", "FilesEditor", cx));
        let input_events = cx.subscribe(&input, |_this: &mut Self, _, event, cx| {
            if matches!(
                event,
                ComposerInputEvent::Edited | ComposerInputEvent::CursorMoved
            ) {
                cx.notify();
            }
        });
        Self {
            engine,
            space_id,
            input,
            doc: EditorDoc::default(),
            focus: cx.focus_handle(),
            task: None,
            _input_events: input_events,
        }
    }

    /// A click in the tree. A dirty buffer is never replaced without asking:
    /// the prompt goes up and the path waits behind it (`prompt`). The same
    /// dirty file clicked again keeps its buffer and takes any prompt down:
    /// clicking the file you are on is choosing to stay. The rule and its
    /// state change are `EditorDoc::click`, tested there.
    pub fn open(&mut self, path: String, cx: &mut Context<Self>) {
        let current = self.input.read(cx).text().to_string();
        match self.doc.click(&current, &path) {
            Switch::Load => self.load(path, cx),
            Switch::Prompt | Switch::Stay => cx.notify(),
        }
    }

    /// The prompt's Save: write, and open the waiting path once the write
    /// lands (`save`'s answer takes it). A refused write drops it. When no
    /// write can start (a conflict banner is up: the owner settles that
    /// first) the prompt steps aside rather than sit there doing nothing.
    fn save_then_open(&mut self, cx: &mut Context<Self>) {
        self.save(cx);
        if !self.doc.saving {
            self.keep_editing(cx);
        }
    }

    /// The prompt's Discard: the waiting path loads over the edits.
    fn discard_then_open(&mut self, cx: &mut Context<Self>) {
        if let Some(path) = self.doc.take_pending_open() {
            self.load(path, cx);
        }
    }

    /// The prompt's Keep editing: the prompt goes, the buffer stays.
    fn keep_editing(&mut self, cx: &mut Context<Self>) {
        self.doc.keep_editing();
        cx.notify();
    }

    /// Replace the buffer with `path`'s read. Only `open` and the prompt's
    /// answers get here, after the dirty check.
    fn load(&mut self, path: String, cx: &mut Context<Self>) {
        self.doc.loading(&path);
        self.input.update(cx, |input, cx| input.set_text("", cx));
        let engine = self.engine.clone();
        let params = serde_json::json!({
            "spaceId": self.space_id,
            "path": &path,
            "range": Option::<LineRange>::None,
        });
        self.task = Some(cx.spawn(async move |this, cx| {
            let result = engine.client().call(methods::FILES_READ, params).await;
            let _ = this.update(cx, |this, cx| {
                match result.and_then(|v| {
                    serde_json::from_value::<FileRead>(v)
                        .map_err(|e| zeron_rpc::RpcError::Failed(e.to_string()))
                }) {
                    Ok(read) => {
                        if let Some(text) = this.doc.opened(&path, read) {
                            this.input.update(cx, |input, cx| input.set_text(text, cx));
                        }
                    }
                    Err(err) => this.doc.failed(err.to_string()),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    pub fn save(&mut self, cx: &mut Context<Self>) {
        let text = self.input.read(cx).text().to_string();
        let Some((path, expected_hash)) = self.doc.save_request(&text) else {
            return;
        };
        let engine = self.engine.clone();
        let params = serde_json::json!({
            "spaceId": self.space_id,
            "path": &path,
            "content": &text,
            "expectedHash": expected_hash,
        });
        self.task = Some(cx.spawn(async move |this, cx| {
            let result = engine.client().call(methods::FILES_WRITE, params).await;
            let _ = this.update(cx, |this, cx| {
                match result.and_then(|v| {
                    serde_json::from_value::<FileWrite>(v)
                        .map_err(|e| zeron_rpc::RpcError::Failed(e.to_string()))
                }) {
                    Ok(write) => {
                        if this.doc.write_answered(write, text) == SaveOutcome::Saved {
                            cx.emit(EditorEvent::Saved(path));
                            if let Some(next) = this.doc.take_pending_open() {
                                this.load(next, cx);
                            }
                        }
                    }
                    Err(err) => this.doc.failed(err.to_string()),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn reload_from_disk(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = self.doc.take_theirs() {
            self.input.update(cx, |input, cx| input.set_text(text, cx));
        }
        cx.notify();
    }

    fn overwrite(&mut self, cx: &mut Context<Self>) {
        if self.doc.keep_mine() {
            self.save(cx);
        }
    }

    fn header(&self, theme: &Theme, cx: &Context<Self>) -> gpui::Div {
        let text = self.input.read(cx).text();
        let dirty = self.doc.is_dirty(text);
        let line = text[..self.input.read(cx).cursor_offset().min(text.len())]
            .matches('\n')
            .count()
            + 1;
        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .h(px(28.0))
            .px(px(10.0))
            .border_b_1()
            .border_color(theme.border) // TOKEN: surya.pane.divider
            .text_size(px(12.0))
            .text_color(theme.text_muted)
            .child(
                div()
                    .text_color(theme.text)
                    .child(self.doc.path.clone().unwrap_or_else(|| "No file".into())),
            )
            .when(dirty, |d| {
                d.child(div().text_color(theme.warning).child("●"))
            })
            .when(self.doc.saving, |d| d.child("saving…"))
            .child(div().flex_1())
            .when(self.doc.body == Body::Text, |d| {
                d.child(format!("Ln {line}"))
            })
            .when_some(self.doc.note.clone(), |d, note| {
                d.child(div().text_color(theme.danger).child(note))
            })
    }

    fn banner(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<gpui::Div> {
        let conflict = self.doc.conflict.as_ref()?;
        let button = |label: &'static str, theme: &Theme| {
            div()
                .px(px(8.0))
                .py(px(2.0))
                .rounded(px(4.0))
                .bg(theme.surface_raised)
                .hover(|d| d.bg(theme.element_hover))
                .cursor_pointer()
                .child(label)
        };
        Some(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .px(px(10.0))
                .py(px(6.0))
                .bg(theme.danger_muted) // TOKEN: surya.banner.conflict
                .text_size(px(12.0))
                .text_color(theme.text)
                .child(format!("Save refused: {}", conflict.reason))
                .child(div().flex_1())
                .child(button("Reload from disk", theme).on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|this, _, _, cx| this.reload_from_disk(cx)),
                ))
                .child(button("Overwrite", theme).on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|this, _, _, cx| this.overwrite(cx)),
                )),
        )
    }

    /// The unsaved-edits prompt, in the pane above the buffer. Three ways
    /// out, all explicit: nothing here is a native dialog.
    fn prompt(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<gpui::Div> {
        let next = self.doc.prompt_for(self.input.read(cx).text())?.to_string();
        let here = self.doc.path.clone().unwrap_or_default();
        Some(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(8.0))
                .px(px(10.0))
                .py(px(6.0))
                .bg(theme.surface_raised) // TOKEN: surya.banner.ask
                .border_b_1()
                .border_color(theme.border)
                .text_size(px(12.0))
                .text_color(theme.text)
                .child(format!("{here} has unsaved changes. Save or discard them before opening {next}?"))
                .child(div().flex_1())
                .child(
                    crate::popover::btn_ghost(theme, "Keep editing", "files-editor-keep")
                        .id("files-editor-keep")
                        .on_click(cx.listener(|this, _, _, cx| this.keep_editing(cx))),
                )
                .child(
                    crate::popover::btn_danger(theme, "Discard")
                        .id("files-editor-discard")
                        .on_click(cx.listener(|this, _, _, cx| this.discard_then_open(cx))),
                )
                .child(
                    crate::popover::btn_primary(theme, "Save")
                        .id("files-editor-save-then-open")
                        .on_click(cx.listener(|this, _, _, cx| this.save_then_open(cx))),
                ),
        )
    }

    fn placeholder(&self, theme: &Theme) -> Option<gpui::Div> {
        let text = match &self.doc.body {
            Body::Text => return None,
            Body::Empty => "Select a file in the tree".to_string(),
            Body::Loading => "Loading…".to_string(),
            Body::Binary { size } => format!("Binary file, {size} bytes. No text view."),
            Body::TooLarge { size, max } => {
                format!("{size} bytes is over the {max} byte read cap.")
            }
            Body::Error(why) => format!("Could not open: {why}"),
        };
        Some(
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(13.0))
                .text_color(theme.text_faint)
                .child(text),
        )
    }
}

impl Focusable for FileEditor {
    fn focus_handle(&self, cx: &gpui::App) -> FocusHandle {
        if self.doc.body == Body::Text {
            self.input.read(cx).focus_handle(cx)
        } else {
            self.focus.clone()
        }
    }
}

impl Render for FileEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let header = self.header(&theme, cx);
        let banner = self.banner(&theme, cx);
        let prompt = self.prompt(&theme, cx);
        let placeholder = self.placeholder(&theme);
        div()
            .id("files-editor")
            .track_focus(&self.focus)
            // Dispatched along the focus path from the buffer up, so it
            // lands here while the buffer holds focus; handled, it stops,
            // and the sidebar toggle bound to the same chord never runs.
            .on_action(cx.listener(|this, _: &SaveFile, _, cx| this.save(cx)))
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.surface) // TOKEN: surya.editor.bg
            .child(header)
            .children(prompt)
            .children(banner)
            .when_some(placeholder, |d, p| d.child(p))
            .when(self.doc.body == Body::Text, |d| {
                d.child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .p(px(8.0))
                        .font_family("Geist Mono") // TOKEN: surya.font.mono
                        .text_size(px(13.0))
                        .child(self.input.clone()),
                )
            })
    }
}
