//! Files pane: haktui's tree and editor as two gpui views over the engine's
//! file RPCs, side by side in a [`FilesPane`]. Self-contained on purpose:
//! nothing in `shell.rs` knows about it yet; wave 3 mounts the pane in the
//! right column. `run_demo` opens the pane alone against a local engine.
//!
//! Mapping to haktui: `crates/haktui/src/files.rs` (tree model + rows) is
//! [`model`] + [`tree`]; `editor.rs` (tabs, conflict, take-theirs /
//! keep-mine) is [`editor`] with one document instead of tabs.

pub mod demo;
pub mod editor;
pub mod editor_doc;
pub mod model;
pub mod tree;

use gpui::{
    Context, Entity, FocusHandle, Focusable, IntoElement, KeyBinding, Render, Subscription, Window,
    div, prelude::*, px,
};

use crate::composer;
use crate::state::EngineHandle;
use crate::theme::Theme;
pub use demo::run_demo;
pub use editor::{EditorEvent, FileEditor};
pub use tree::{FileTreeView, TreeEvent};

/// Key context of the editor's buffer: the composer's editing keys, but
/// Enter inserts a newline and nothing submits. Call once at app start,
/// after `composer::init`.
pub fn init(cx: &mut gpui::App) {
    use composer::*;
    let ctx = Some("FilesEditor");
    let mut bindings = vec![
        KeyBinding::new("enter", Newline, ctx),
        KeyBinding::new("shift-enter", Newline, ctx),
        KeyBinding::new("backspace", Backspace, ctx),
        KeyBinding::new("delete", Delete, ctx),
        KeyBinding::new("left", Left, ctx),
        KeyBinding::new("right", Right, ctx),
        KeyBinding::new("up", Up, ctx),
        KeyBinding::new("down", Down, ctx),
        KeyBinding::new("shift-left", SelectLeft, ctx),
        KeyBinding::new("shift-right", SelectRight, ctx),
        KeyBinding::new("shift-up", SelectUp, ctx),
        KeyBinding::new("shift-down", SelectDown, ctx),
        KeyBinding::new("home", Home, ctx),
        KeyBinding::new("end", End, ctx),
        KeyBinding::new("shift-home", SelectHome, ctx),
        KeyBinding::new("shift-end", SelectEnd, ctx),
        KeyBinding::new("cmd-left", Home, ctx),
        KeyBinding::new("cmd-right", End, ctx),
        KeyBinding::new("cmd-up", DocStart, ctx),
        KeyBinding::new("cmd-down", DocEnd, ctx),
        KeyBinding::new("shift-cmd-up", SelectDocStart, ctx),
        KeyBinding::new("shift-cmd-down", SelectDocEnd, ctx),
        KeyBinding::new("ctrl-home", DocStart, ctx),
        KeyBinding::new("ctrl-end", DocEnd, ctx),
        KeyBinding::new("cmd-backspace", DeleteToLineStart, ctx),
        KeyBinding::new("cmd-delete", DeleteToLineEnd, ctx),
    ];
    for prefix in ["cmd", "ctrl"] {
        bindings.push(KeyBinding::new(&format!("{prefix}-z"), Undo, ctx));
        bindings.push(KeyBinding::new(&format!("shift-{prefix}-z"), Redo, ctx));
        bindings.push(KeyBinding::new(&format!("{prefix}-a"), SelectAll, ctx));
        bindings.push(KeyBinding::new(&format!("{prefix}-c"), Copy, ctx));
        bindings.push(KeyBinding::new(&format!("{prefix}-x"), Cut, ctx));
        bindings.push(KeyBinding::new(&format!("{prefix}-v"), Paste, ctx));
    }
    let word = if cfg!(target_os = "macos") {
        "alt"
    } else {
        "ctrl"
    };
    bindings.push(KeyBinding::new(
        &format!("{word}-backspace"),
        DeleteWordLeft,
        ctx,
    ));
    bindings.push(KeyBinding::new(
        &format!("{word}-delete"),
        DeleteWordRight,
        ctx,
    ));
    bindings.push(KeyBinding::new(&format!("{word}-left"), WordLeft, ctx));
    bindings.push(KeyBinding::new(&format!("{word}-right"), WordRight, ctx));
    bindings.push(KeyBinding::new(
        &format!("shift-{word}-left"),
        SelectWordLeft,
        ctx,
    ));
    bindings.push(KeyBinding::new(
        &format!("shift-{word}-right"),
        SelectWordRight,
        ctx,
    ));
    cx.bind_keys(bindings);
}

/// Tree on the left, editor on the right, one space.
pub struct FilesPane {
    pub tree: Entity<FileTreeView>,
    pub editor: Entity<FileEditor>,
    tree_width: f32,
    _tree_events: Subscription,
}

impl FilesPane {
    pub fn new(engine: EngineHandle, space_id: String, cx: &mut Context<Self>) -> Self {
        let tree = cx.new(|cx| FileTreeView::new(engine.clone(), space_id.clone(), cx));
        let editor = cx.new(|cx| FileEditor::new(engine, space_id, cx));
        let editor_for_tree = editor.clone();
        let tree_events = cx.subscribe(&tree, move |_this: &mut Self, _, event, cx| match event {
            TreeEvent::Open(path) => {
                let path = path.clone();
                editor_for_tree.update(cx, |editor, cx| editor.open(path, cx));
                cx.notify();
            }
        });
        Self {
            tree,
            editor,
            tree_width: 240.0,
            _tree_events: tree_events,
        }
    }
}

impl Focusable for FilesPane {
    fn focus_handle(&self, cx: &gpui::App) -> FocusHandle {
        self.tree.read(cx).focus_handle(cx)
    }
}

impl Render for FilesPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx);
        div()
            .flex()
            .size_full()
            .bg(theme.bg)
            .child(
                div()
                    .w(px(self.tree_width))
                    .h_full()
                    .border_r_1()
                    .border_color(theme.border) // TOKEN: surya.pane.divider
                    .child(self.tree.clone()),
            )
            .child(div().flex_1().min_w_0().h_full().child(self.editor.clone()))
    }
}
