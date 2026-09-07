//! File-tree action controls use the selected row as their target.
use super::*;
use crate::files::FileAction;
use crate::popover;

impl FileTreeView {
    pub(super) fn action_bar(&self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let selected = self
            .rows()
            .into_iter()
            .find(|row| Some(&row.path) == self.model.selected.as_ref());
        let dir = selected
            .as_ref()
            .map(|row| {
                if row.kind == FileKind::Dir {
                    row.path.clone()
                } else {
                    crate::files::actions::parent(&row.path).to_string()
                }
            })
            .unwrap_or_default();
        let file = selected
            .filter(|row| row.kind == FileKind::File)
            .map(|row| row.path);
        div()
            .flex()
            .flex_wrap()
            .gap(px(4.0))
            .px(px(6.0))
            .py(px(4.0))
            .child(
                popover::btn_ghost(theme, "New file", "tree-new-file")
                    .id("tree-new-file")
                    .on_click(cx.listener(move |_, _, _, cx| {
                        cx.emit(TreeEvent::Action(FileAction::Create(dir.clone())))
                    })),
            )
            .when_some(file, |el, path| {
                let rename = path.clone();
                el.child(
                    popover::btn_ghost(theme, "Rename", "tree-rename-file")
                        .id("tree-rename-file")
                        .on_click(cx.listener(move |_, _, _, cx| {
                            cx.emit(TreeEvent::Action(FileAction::Rename(rename.clone())))
                        })),
                )
                .child(
                    popover::btn_ghost(theme, "Delete", "tree-delete-file")
                        .id("tree-delete-file")
                        .on_click(cx.listener(move |_, _, _, cx| {
                            cx.emit(TreeEvent::Action(FileAction::Delete(path.clone())))
                        })),
                )
            })
            .into_any_element()
    }
}
