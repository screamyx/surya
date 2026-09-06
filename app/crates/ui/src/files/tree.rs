//! The file tree view: haktui's column-4 tree on `FilesTree`, `FilesWatch`
//! and `FilesSearch`. Rows come from [`TreeModel`]; this file only asks the
//! engine, folds answers in, draws, and routes keys and clicks.
//!
//! Keys (tree focused): up / down move, right expands or steps into a
//! directory, left collapses or steps to the parent, enter opens (a file)
//! or toggles (a directory), escape clears the filter. Typing in the filter
//! box switches the list to `FilesSearch` results; enter there opens the
//! highlighted match.

use std::collections::HashMap;
use std::sync::Arc;

use gpui::{
    AnyElement, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement, KeyDownEvent,
    Render, ScrollStrategy, Subscription, Task, UniformListScrollHandle, Window, div, prelude::*,
    px,
};
use surya_proto::FileSearchMatch;
use surya_proto::files::{FileKind, FileTree, FileWatchBatch};
use surya_rpc::methods;

use super::model::{Row, TreeModel};
use crate::composer::{ComposerInput, ComposerInputEvent};
use crate::state::EngineHandle;
use crate::theme::Theme;

pub enum TreeEvent {
    /// A file row was activated.
    Open(String),
}

pub struct FileTreeView {
    engine: EngineHandle,
    space_id: String,
    pub model: TreeModel,
    filter: Entity<ComposerInput>,
    /// `FilesSearch` answers for the current filter text; `None` = no filter.
    matches: Option<Vec<FileSearchMatch>>,
    focus: FocusHandle,
    scroll: UniformListScrollHandle,
    /// One in-flight listing per directory; a newer request for the same
    /// directory replaces (cancels) the older one, never a different one.
    load_tasks: HashMap<String, Task<()>>,
    search_task: Option<Task<()>>,
    _watch_task: Task<()>,
    _filter_events: Subscription,
    pub loads_asked: u32,
    pub loads_answered: u32,
    pub watch_batches: u32,
}

impl EventEmitter<TreeEvent> for FileTreeView {}

impl FileTreeView {
    pub fn new(engine: EngineHandle, space_id: String, cx: &mut Context<Self>) -> Self {
        let filter = cx.new(|cx| ComposerInput::new("Filter files…", cx));
        let filter_events = cx.subscribe(&filter, |this: &mut Self, _, event, cx| match event {
            ComposerInputEvent::Edited => this.filter_changed(cx),
            ComposerInputEvent::Submitted => this.activate_selected(cx),
            _ => {}
        });
        let watch_task = Self::spawn_watch(engine.clone(), space_id.clone(), cx);
        let mut this = Self {
            engine,
            space_id,
            model: TreeModel::default(),
            filter,
            matches: None,
            focus: cx.focus_handle(),
            scroll: UniformListScrollHandle::new(),
            load_tasks: HashMap::new(),
            search_task: None,
            _watch_task: watch_task,
            _filter_events: filter_events,
            loads_asked: 0,
            loads_answered: 0,
            watch_batches: 0,
        };
        this.load("", cx);
        this
    }

    /// List one directory (depth 1) and fold the answer in.
    pub fn load(&mut self, dir: &str, cx: &mut Context<Self>) {
        self.loads_asked += 1;
        let engine = self.engine.clone();
        let params = serde_json::json!({ "spaceId": self.space_id, "path": dir, "depth": 1 });
        let key = dir.to_string();
        let done_key = key.clone();
        let task = cx.spawn(async move |this, cx| {
            let result = engine.client().call(methods::FILES_TREE, params).await;
            let _ = this.update(cx, |this, cx| {
                this.loads_answered += 1;
                if let Ok(tree) = result.and_then(|v| {
                    serde_json::from_value::<FileTree>(v)
                        .map_err(|e| surya_rpc::RpcError::Failed(e.to_string()))
                }) {
                    this.model.apply_tree(&tree);
                }
                this.load_tasks.remove(&done_key);
                cx.notify();
            });
        });
        self.load_tasks.insert(key, task);
    }

    fn spawn_watch(engine: EngineHandle, space_id: String, cx: &mut Context<Self>) -> Task<()> {
        cx.spawn(async move |this, cx| {
            const RETRY: std::time::Duration = std::time::Duration::from_secs(2);
            loop {
                let params = serde_json::json!({ "spaceId": space_id });
                let mut rx = match engine
                    .client()
                    .subscribe(methods::FILES_WATCH, params)
                    .await
                {
                    Ok(rx) => rx,
                    Err(_) => {
                        if this.update(cx, |_, _| {}).is_err() {
                            return;
                        }
                        cx.background_executor().timer(RETRY).await;
                        continue;
                    }
                };
                while let Some(value) = rx.recv().await {
                    let Ok(batch) = serde_json::from_value::<FileWatchBatch>(value) else {
                        continue;
                    };
                    let alive = this.update(cx, |this, cx| {
                        this.watch_batches += 1;
                        for dir in this.model.apply_watch(&batch) {
                            this.load(&dir, cx);
                        }
                        cx.notify();
                    });
                    if alive.is_err() {
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

    fn filter_changed(&mut self, cx: &mut Context<Self>) {
        let query = self.filter.read(cx).text().trim().to_string();
        if query.is_empty() {
            self.matches = None;
            self.search_task = None;
            cx.notify();
            return;
        }
        let engine = self.engine.clone();
        let params = serde_json::json!({ "spaceId": self.space_id, "query": query });
        self.search_task = Some(cx.spawn(async move |this, cx| {
            let result = engine.client().call(methods::FILES_SEARCH, params).await;
            let _ = this.update(cx, |this, cx| {
                // A stale answer for a cleared box must not resurrect the list.
                if this.filter.read(cx).text().trim().is_empty() {
                    return;
                }
                let matches = result
                    .ok()
                    .and_then(|v| serde_json::from_value::<Vec<FileSearchMatch>>(v).ok())
                    .unwrap_or_default();
                this.model.selected = matches.first().map(|m| m.path.clone());
                this.matches = Some(matches);
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// The rows on screen: search matches while filtering, else the tree.
    fn rows(&self) -> Vec<Row> {
        match &self.matches {
            Some(matches) => matches
                .iter()
                .map(|m| Row {
                    path: m.path.clone(),
                    kind: if m.is_dir {
                        FileKind::Dir
                    } else {
                        FileKind::File
                    },
                    status: String::new(),
                    depth: 0,
                    label: m.path.clone(),
                    expanded: false,
                    has_children: m.is_dir,
                })
                .collect(),
            None => self.model.rows(),
        }
    }

    fn activate(&mut self, path: &str, cx: &mut Context<Self>) {
        let is_dir = self
            .model
            .entry(path)
            .map(|e| e.kind == FileKind::Dir)
            .or_else(|| {
                self.matches
                    .as_ref()
                    .and_then(|m| m.iter().find(|m| m.path == path).map(|m| m.is_dir))
            })
            .unwrap_or(false);
        self.model.selected = Some(path.to_string());
        if is_dir {
            if self.matches.is_some() {
                // Jump out of the filter to the directory itself.
                self.clear_filter(cx);
                self.reveal(path, cx);
            } else if self.model.toggle(path) {
                self.load(path, cx);
            }
        } else {
            cx.emit(TreeEvent::Open(path.to_string()));
        }
        cx.notify();
    }

    fn activate_selected(&mut self, cx: &mut Context<Self>) {
        if let Some(path) = self.model.selected.clone() {
            self.activate(&path, cx);
        }
    }

    /// Expand every ancestor of `path` so its row shows.
    fn reveal(&mut self, path: &str, cx: &mut Context<Self>) {
        let mut acc = String::new();
        for part in path.split('/') {
            if !acc.is_empty() {
                acc.push('/');
            }
            acc.push_str(part);
            if acc != path && self.model.expand(&acc) {
                self.load(&acc, cx);
            }
        }
    }

    fn clear_filter(&mut self, cx: &mut Context<Self>) {
        self.filter.update(cx, |input, cx| input.set_text("", cx));
        self.matches = None;
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let rows = self.rows();
        let filter_focused = self.filter.read(cx).focus_handle(cx).is_focused(window);
        match key {
            "up" | "down" => {
                let delta = if key == "up" { -1 } else { 1 };
                if let Some(sel) = self.model.move_selection(&rows, delta)
                    && let Some(ix) = rows.iter().position(|r| r.path == sel)
                {
                    self.scroll.scroll_to_item(ix, ScrollStrategy::Nearest);
                }
                cx.notify();
            }
            "enter" if !filter_focused => self.activate_selected(cx),
            "escape" => {
                self.clear_filter(cx);
                self.focus.focus(window, cx);
                cx.notify();
            }
            "right" if !filter_focused => {
                let Some(sel) = self.model.selected.clone() else {
                    return;
                };
                if let Some(row) = rows.iter().find(|r| r.path == sel)
                    && row.kind == FileKind::Dir
                {
                    if row.expanded {
                        self.model.move_selection(&rows, 1);
                    } else if self.model.expand(&sel) {
                        self.load(&sel, cx);
                    }
                    cx.notify();
                }
            }
            "left" if !filter_focused => {
                let Some(sel) = self.model.selected.clone() else {
                    return;
                };
                if self.model.is_expanded(&sel) {
                    self.model.collapse(&sel);
                } else if let Some((parent, _)) = sel.rsplit_once('/') {
                    self.model.selected = Some(parent.to_string());
                }
                cx.notify();
            }
            _ => {}
        }
    }

    fn render_row(
        &self,
        ix: usize,
        row: &Row,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let selected = self.model.selected.as_deref() == Some(row.path.as_str());
        let status_color = match row.status.as_str() {
            "M" => theme.warning,
            "A" => theme.success,
            "D" => theme.danger,
            "?" => theme.text_faint,
            _ => theme.text_muted,
        };
        let chevron = match (row.kind, row.expanded, row.has_children) {
            (FileKind::Dir, true, _) => "▾",
            (FileKind::Dir, false, true) => "▸",
            (FileKind::Dir, false, false) => "·",
            _ => " ",
        };
        let path = row.path.clone();
        div()
            .id(ix)
            .flex()
            .items_center()
            .gap(px(4.0))
            .h(px(22.0))
            .pl(px(8.0 + row.depth as f32 * 12.0))
            .pr(px(8.0))
            .text_size(px(12.0))
            .text_color(if row.status == "?" {
                theme.text_muted
            } else {
                theme.text
            })
            .when(selected, |d| d.bg(theme.element_active)) // TOKEN: surya.tree.selected
            .hover(|d| d.bg(theme.element_hover))
            .cursor_pointer()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, _, _, cx| this.activate(&path, cx)),
            )
            .child(
                div()
                    .w(px(10.0))
                    .text_color(theme.text_faint)
                    .child(chevron),
            )
            .child(div().flex_1().truncate().child(row.label.clone()))
            .when(!row.status.is_empty(), |d| {
                d.child(div().text_color(status_color).child(row.status.clone()))
            })
            .into_any_element()
    }
}

impl Focusable for FileTreeView {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for FileTreeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let rows = Arc::new(self.rows());
        let count = rows.len();
        let entity = cx.entity();
        let row_theme = theme.clone();
        let list = gpui::uniform_list("files-tree", count, move |range, _window, app| {
            entity.update(app, |this, cx| {
                range
                    .filter_map(|ix| {
                        rows.get(ix)
                            .map(|row| this.render_row(ix, row, &row_theme, cx))
                    })
                    .collect::<Vec<AnyElement>>()
            })
        })
        .size_full()
        .track_scroll(&self.scroll);
        let empty = if count == 0 {
            Some(
                div()
                    .p(px(10.0))
                    .text_size(px(12.0))
                    .text_color(theme.text_faint)
                    .child(if self.matches.is_some() {
                        "No matches"
                    } else {
                        "Empty"
                    }),
            )
        } else {
            None
        };
        div()
            .id("files-tree-pane")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.on_key_down(event, window, cx)
            }))
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.surface) // TOKEN: surya.tree.bg
            .child(
                div()
                    .p(px(6.0))
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .px(px(8.0))
                            .py(px(2.0))
                            .rounded(px(6.0))
                            .bg(theme.input_bg)
                            .text_size(px(12.0))
                            .child(self.filter.clone()),
                    ),
            )
            .children(empty)
            .child(div().flex_1().min_h_0().child(list))
    }
}
