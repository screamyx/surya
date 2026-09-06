//! The "Add a task" box on the Queued column: its submit, and the two keys the
//! input itself will not handle.
//!
//! The box is a [`ComposerInput`] built with the `PaletteSearch` key context
//! (`board.rs`). That context binds text-editing keys only: enter and escape
//! are deliberately left unbound so they bubble to the frame that owns the
//! input (`composer.rs`, "navigation keys ... bubble to the palette frame
//! instead"). This pane is that frame, so the two keys are handled here.

use gpui::{Context, Focusable, KeyDownEvent, Window};
use surya_proto::TaskStatus;

use super::board::TasksPane;

impl TasksPane {
    /// Enter submits, escape clears and hands focus back to the board.
    ///
    /// Returns whether the key was ours. The caller runs this BEFORE its own
    /// "is the pane focused" gate: when the box has the caret the pane root
    /// does not, so that gate would drop these keys on the floor.
    pub(super) fn on_quick_add_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.quick_add.focus_handle(cx).is_focused(window) {
            return false;
        }
        if event.keystroke.modifiers.modified() {
            return false;
        }
        match event.keystroke.key.as_str() {
            "enter" => {
                self.submit_quick_add(cx);
                true
            }
            "escape" => {
                self.quick_add
                    .update(cx, |input, cx| input.set_text(String::new(), cx));
                window.focus(&self.focus_handle, cx);
                cx.notify();
                true
            }
            _ => false,
        }
    }

    /// Create the typed task and empty the box. A blank title is a no-op, so
    /// enter on an empty box costs nothing.
    pub(super) fn submit_quick_add(&mut self, cx: &mut Context<Self>) {
        let title = self.quick_add.read(cx).text().trim().to_string();
        if title.is_empty() {
            return;
        }
        self.quick_add
            .update(cx, |input, cx| input.set_text(String::new(), cx));
        self.create_task(&title, TaskStatus::Queued, serde_json::Value::Null, cx);
    }
}
