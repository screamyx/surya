//! Files pane: haktui's tree and editor as two gpui views over the engine's
//! file RPCs, side by side in a [`FilesPane`]. Self-contained on purpose:
//! nothing in `shell.rs` knows about it yet; wave 3 mounts the pane in the
//! right column. `run_demo` opens the pane alone against a local engine.
//!
//! Mapping to haktui: `crates/haktui/src/files.rs` (tree model + rows) is
//! [`model`] + [`tree`]; `editor.rs` (tabs, conflict, take-theirs /
//! keep-mine) is [`editor`] with one document instead of tabs.

mod actions;
pub use actions::FileAction;
pub mod demo;
pub mod editor;
pub mod editor_doc;
pub mod model;
pub mod notice;
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

gpui::actions!(files, [SaveFile]);

/// Key context of the editor's buffer: the composer's editing keys, but
/// Enter inserts a newline and nothing submits. Call once at app start,
/// after `composer::init`.
pub fn init(cx: &mut gpui::App) {
    cx.bind_keys(files_key_bindings());
}

/// Bind Cmd+S / Ctrl+S to [`SaveFile`] in the editor's context. Call AFTER
/// every chord a keymap can remap (the shell's `apply_keymap` calls it last;
/// an app that skips `apply_keymap`, like the files demo, calls it itself),
/// never before: gpui scores a binding
/// with no context at the full depth of the context stack, the same score a
/// binding matched on the innermost context gets, and breaks that tie by
/// binding order, later wins (gpui `keymap.rs`, `bindings_for_input` and
/// `binding_enabled`). Bound first, the editor's save lost to the shell's
/// `mod-s` ToggleSidebar every time, and Ctrl+S hid the sidebar with the
/// file still dirty (e2e FILES-01, 2026-09-06).
pub fn bind_save_keys(cx: &mut gpui::App) {
    cx.bind_keys(save_key_bindings());
}

/// The save chords as data, so a test can read them without an `App`.
/// Both spellings on every platform, as the composer does for undo: Ctrl+S
/// on a Mac saves too, and costs nothing.
pub fn save_key_bindings() -> Vec<KeyBinding> {
    let ctx = Some("FilesEditor");
    vec![
        KeyBinding::new("cmd-s", SaveFile, ctx),
        KeyBinding::new("ctrl-s", SaveFile, ctx),
    ]
}

/// The FilesEditor keymap as data, so a test can read it without an `App`.
pub fn files_key_bindings() -> Vec<KeyBinding> {
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
    bindings
}

/// Tree on the left, editor on the right, one space.
pub struct FilesPane {
    pub tree: Entity<FileTreeView>,
    pub editor: Entity<FileEditor>,
    tree_width: f32,
    engine: EngineHandle,
    space_id: String,
    action_form: Option<actions::ActionForm>,
    action_focus: FocusHandle,
    _tree_events: Subscription,
}

impl FilesPane {
    /// The editor holds edits not yet written to disk.
    pub fn has_unsaved_edits(&self, cx: &gpui::App) -> bool {
        self.editor.read(cx).is_dirty(cx)
    }

    pub fn new(engine: EngineHandle, space_id: String, cx: &mut Context<Self>) -> Self {
        let tree = cx.new(|cx| FileTreeView::new(engine.clone(), space_id.clone(), cx));
        let editor = cx.new(|cx| FileEditor::new(engine.clone(), space_id.clone(), cx));
        let editor_for_tree = editor.clone();
        let tree_events = cx.subscribe(&tree, move |this: &mut Self, _, event, cx| match event {
            TreeEvent::Action(action) => this.begin_action(action.clone(), cx),
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
            engine,
            space_id,
            action_form: None,
            action_focus: cx.focus_handle(),
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(form) = self.render_action(window, cx) { return form; }
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
            .into_any_element()
    }
}

#[cfg(test)]
mod key_tests {
    use super::*;
    use gpui::{Action as _, KeyContext, Keymap, Keystroke};

    /// The rule the fix rests on, checked against gpui itself: with the
    /// shell's context-less `mod-s` ToggleSidebar bound first and the
    /// editor's `ctrl-s` SaveFile bound after, a Ctrl+S in the FilesEditor
    /// context resolves to SaveFile first. Bound the other way round, the
    /// sidebar wins, which is the bug.
    #[test]
    fn save_outranks_the_sidebar_toggle_only_when_bound_after_it() {
        let sidebar = || KeyBinding::new("ctrl-s", crate::shell::ToggleSidebar, None);
        let stack = [KeyContext::parse("FilesEditor").unwrap()];
        let ctrl_s = [Keystroke::parse("ctrl-s").unwrap()];

        let mut fixed = Keymap::new(vec![sidebar()]);
        fixed.add_bindings(save_key_bindings());
        let (bindings, _) = fixed.bindings_for_input(&ctrl_s, &stack);
        assert_eq!(bindings[0].action().name(), "files::SaveFile", "the order apply_keymap uses");

        let mut broken = Keymap::new(save_key_bindings());
        broken.add_bindings(vec![sidebar()]);
        let (bindings, _) = broken.bindings_for_input(&ctrl_s, &stack);
        assert_eq!(bindings[0].action().name(), "shell::ToggleSidebar", "the order that lost the save");
    }

    #[test]
    fn a_remapped_sidebar_toggle_leaves_ctrl_s_to_the_editor() {
        let stack = [KeyContext::parse("FilesEditor").unwrap()];
        let ctrl_s = [Keystroke::parse("ctrl-s").unwrap()];
        let mut keymap = Keymap::new(vec![KeyBinding::new("ctrl-b", crate::shell::ToggleSidebar, None)]);
        keymap.add_bindings(save_key_bindings());
        let (bindings, _) = keymap.bindings_for_input(&ctrl_s, &stack);
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].action().name(), "files::SaveFile");
    }

    #[test]
    fn outside_the_editor_ctrl_s_is_still_the_sidebar() {
        let stack = [KeyContext::parse("Composer").unwrap()];
        let ctrl_s = [Keystroke::parse("ctrl-s").unwrap()];
        let mut keymap = Keymap::new(vec![KeyBinding::new("ctrl-s", crate::shell::ToggleSidebar, None)]);
        keymap.add_bindings(save_key_bindings());
        let (bindings, _) = keymap.bindings_for_input(&ctrl_s, &stack);
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].action().name(), "shell::ToggleSidebar");
    }

    #[test]
    fn enter_is_newline_in_the_files_editor_context() {
        // `KeyBinding::new` panics on an unparseable combo, so building the
        // table is itself the parse check.
        let bindings = files_key_bindings();
        let enter = Keystroke::parse("enter").unwrap();
        let hits: Vec<&KeyBinding> = bindings
            .iter()
            .filter(|b| {
                b.keystrokes()
                    .iter()
                    .map(|k| k.inner().clone())
                    .collect::<Vec<_>>()
                    == vec![enter.clone()]
            })
            .collect();
        println!("files keys: bindings={} enter_hits={}", bindings.len(), hits.len());
        assert_eq!(hits.len(), 1, "exactly one Enter binding");
        assert_eq!(hits[0].action().name(), composer::Newline.name());
        let predicate = format!("{:?}", hits[0].predicate());
        assert!(predicate.contains("FilesEditor"), "{predicate}");
        assert!(
            bindings
                .iter()
                .all(|b| format!("{:?}", b.predicate()).contains("FilesEditor")),
            "every files binding is scoped to the FilesEditor context"
        );
    }
}
