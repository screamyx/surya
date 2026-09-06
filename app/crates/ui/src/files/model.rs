//! The file tree's pure model: what the engine has told us about a checkout
//! (`FilesTree` answers, `FilesWatch` batches), which directories are open,
//! and the rows that fall out. No gpui here, so every rule is unit-tested.
//!
//! Ported from haktui `files.rs::visible`: a directory whose only child is a
//! directory folds into it, so `src/main/java/com` is one row, not four rows
//! of nothing. The folded row carries the deepest directory's path.

use std::collections::{BTreeMap, HashMap, HashSet};

use surya_proto::files::{FileEntry, FileEventKind, FileKind, FileTree, FileWatchBatch};

/// One drawable row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub path: String,
    pub kind: FileKind,
    pub status: String,
    pub depth: usize,
    /// The name, or `a/b/c` for a folded chain.
    pub label: String,
    pub expanded: bool,
    /// Directory known (or believed) to have children.
    pub has_children: bool,
}

#[derive(Debug, Default)]
pub struct TreeModel {
    /// Everything seen so far, sorted by path.
    entries: BTreeMap<String, FileEntry>,
    /// Directories whose children have been listed.
    loaded: HashSet<String>,
    expanded: HashSet<String>,
    pub selected: Option<String>,
    /// Watch batches folded in, as a pair with `applied`.
    pub events_seen: usize,
    pub applied: usize,
}

fn parent_of(path: &str) -> &str {
    path.rsplit_once('/').map(|(p, _)| p).unwrap_or("")
}

fn is_under(path: &str, dir: &str) -> bool {
    dir.is_empty() || path.starts_with(&format!("{dir}/"))
}

impl TreeModel {
    /// Fold a `FilesTree` answer in: the listed directory's subtree is
    /// replaced by what the engine sent (a re-list after a watch event drops
    /// what vanished), everything else stays.
    pub fn apply_tree(&mut self, tree: &FileTree) {
        let dir = tree.path.as_str();
        // Only the levels the answer covers are authoritative: a depth-1
        // list of `src` says nothing about `src/deep/lib.rs`.
        let listed_dirs: HashSet<&str> = std::iter::once(dir)
            .chain(
                tree.entries
                    .iter()
                    .filter(|e| e.kind == FileKind::Dir && !e.has_children_at_edge())
                    .map(|e| e.path.as_str()),
            )
            .collect();
        self.entries
            .retain(|path, _| !is_under(path, dir) || !listed_dirs.contains(parent_of(path)));
        for entry in &tree.entries {
            self.entries.insert(entry.path.clone(), entry.clone());
        }
        self.loaded.insert(dir.to_string());
        for d in listed_dirs {
            self.loaded.insert(d.to_string());
        }
    }

    /// Fold a watch batch in. Creates and deletes of known paths are applied
    /// in place; a rename is a delete plus a create. Returns the directories
    /// whose listing should be refreshed (status and `hasChildren` are only
    /// the engine's to know) - deduplicated, only ones already loaded.
    pub fn apply_watch(&mut self, batch: &FileWatchBatch) -> Vec<String> {
        let mut refresh: Vec<String> = Vec::new();
        let loaded = self.loaded.clone();
        let note = |refresh: &mut Vec<String>, path: &str| {
            let parent = parent_of(path).to_string();
            if loaded.contains(&parent) && !refresh.contains(&parent) {
                refresh.push(parent);
            }
        };
        for ev in &batch.events {
            self.events_seen += 1;
            match ev.kind {
                FileEventKind::Create => {
                    self.upsert_placeholder(&ev.path);
                    note(&mut refresh, &ev.path);
                }
                FileEventKind::Delete => {
                    self.remove_subtree(&ev.path);
                    note(&mut refresh, &ev.path);
                }
                FileEventKind::Rename => {
                    if let Some(from) = &ev.from {
                        self.remove_subtree(from);
                        note(&mut refresh, from);
                    }
                    self.upsert_placeholder(&ev.path);
                    note(&mut refresh, &ev.path);
                }
                FileEventKind::Modify => {
                    note(&mut refresh, &ev.path);
                }
            }
            self.applied += 1;
        }
        if batch.truncated {
            // Too much moved to trust the in-place picture: re-list every
            // open directory.
            for dir in self.loaded.iter() {
                if !refresh.contains(dir) {
                    refresh.push(dir.clone());
                }
            }
        }
        if let Some(sel) = &self.selected
            && !self.entries.contains_key(sel)
        {
            self.selected = None;
        }
        refresh
    }

    fn upsert_placeholder(&mut self, path: &str) {
        if self.entries.contains_key(path) || !self.loaded.contains(parent_of(path)) {
            return;
        }
        // Kind unknown until the re-list answers; a file is the safe guess.
        self.entries.insert(
            path.to_string(),
            FileEntry {
                path: path.to_string(),
                kind: FileKind::File,
                status: "?".into(),
                has_children: false,
                size: 0,
            },
        );
    }

    fn remove_subtree(&mut self, path: &str) {
        self.entries.retain(|p, _| p != path && !is_under(p, path));
        self.loaded.retain(|p| p != path && !is_under(p, path));
        self.expanded.retain(|p| p != path && !is_under(p, path));
    }

    pub fn is_loaded(&self, dir: &str) -> bool {
        self.loaded.contains(dir)
    }

    pub fn is_expanded(&self, dir: &str) -> bool {
        self.expanded.contains(dir)
    }

    pub fn entry(&self, path: &str) -> Option<&FileEntry> {
        self.entries.get(path)
    }

    /// Toggle a directory. Returns true when it is now open and its
    /// children still need listing.
    pub fn toggle(&mut self, dir: &str) -> bool {
        if self.expanded.contains(dir) {
            self.expanded.remove(dir);
            false
        } else {
            self.expanded.insert(dir.to_string());
            !self.loaded.contains(dir)
        }
    }

    pub fn expand(&mut self, dir: &str) -> bool {
        if self.expanded.contains(dir) {
            return false;
        }
        self.expanded.insert(dir.to_string());
        !self.loaded.contains(dir)
    }

    pub fn collapse(&mut self, dir: &str) {
        self.expanded.remove(dir);
    }

    /// The rows to draw, haktui's fold rule included.
    pub fn rows(&self) -> Vec<Row> {
        let mut children: HashMap<&str, Vec<&FileEntry>> = HashMap::new();
        for e in self.entries.values() {
            children.entry(parent_of(&e.path)).or_default().push(e);
        }
        let only_child_dir = |dir: &str| -> Option<&FileEntry> {
            match children.get(dir).map(Vec::as_slice) {
                Some([one]) if one.kind == FileKind::Dir && self.loaded.contains(dir) => Some(one),
                _ => None,
            }
        };
        let mut folded_into: HashMap<&str, &str> = HashMap::new();
        for e in self.entries.values() {
            if e.kind != FileKind::Dir || folded_into.contains_key(e.path.as_str()) {
                continue;
            }
            let mut chain = vec![e];
            let mut tail = e;
            while let Some(next) = only_child_dir(&tail.path) {
                chain.push(next);
                tail = next;
            }
            if chain.len() > 1 {
                for d in &chain[..chain.len() - 1] {
                    folded_into.insert(d.path.as_str(), tail.path.as_str());
                }
            }
        }
        let is_open = |dir: &str| -> bool {
            match folded_into.get(dir) {
                Some(tail) => self.expanded.contains(*tail),
                None => self.expanded.contains(dir),
            }
        };
        let mut rows = Vec::new();
        for e in self.entries.values() {
            if folded_into.contains_key(e.path.as_str()) {
                continue;
            }
            let parent = parent_of(&e.path);
            let mut shown = true;
            let mut acc = String::new();
            if !parent.is_empty() {
                for part in parent.split('/') {
                    if !acc.is_empty() {
                        acc.push('/');
                    }
                    acc.push_str(part);
                    if folded_into.get(acc.as_str()) == Some(&e.path.as_str()) {
                        continue;
                    }
                    if !is_open(&acc) {
                        shown = false;
                        break;
                    }
                }
            }
            if !shown {
                continue;
            }
            let mut head = e.path.as_str();
            while let Some((p, _)) = head.rsplit_once('/') {
                if folded_into.get(p) == Some(&e.path.as_str()) {
                    head = p;
                } else {
                    break;
                }
            }
            let mut depth = 0;
            if !parent.is_empty() {
                let mut acc = String::new();
                for part in parent.split('/') {
                    if !acc.is_empty() {
                        acc.push('/');
                    }
                    acc.push_str(part);
                    if !folded_into.contains_key(acc.as_str()) {
                        depth += 1;
                    }
                }
            }
            let label = if head == e.path {
                e.path.rsplit('/').next().unwrap_or(&e.path).to_string()
            } else {
                let head_parent_len = head.rsplit_once('/').map(|(p, _)| p.len() + 1).unwrap_or(0);
                e.path[head_parent_len..].to_string()
            };
            let has_children = e.kind == FileKind::Dir
                && (e.has_children || children.get(e.path.as_str()).is_some_and(|c| !c.is_empty()));
            rows.push(Row {
                path: e.path.clone(),
                kind: e.kind,
                status: e.status.clone(),
                depth,
                label,
                expanded: e.kind == FileKind::Dir && self.expanded.contains(&e.path),
                has_children,
            });
        }
        rows
    }

    /// Move the selection by `delta` rows; returns the new selection.
    pub fn move_selection(&mut self, rows: &[Row], delta: isize) -> Option<String> {
        if rows.is_empty() {
            self.selected = None;
            return None;
        }
        let current = self
            .selected
            .as_ref()
            .and_then(|s| rows.iter().position(|r| &r.path == s));
        let next = match current {
            None => {
                if delta >= 0 {
                    0
                } else {
                    rows.len() - 1
                }
            }
            Some(i) => (i as isize + delta).clamp(0, rows.len() as isize - 1) as usize,
        };
        self.selected = Some(rows[next].path.clone());
        self.selected.clone()
    }
}

trait EdgeExt {
    fn has_children_at_edge(&self) -> bool;
}

impl EdgeExt for FileEntry {
    /// The engine sets `hasChildren` only on directories at the depth edge,
    /// the ones whose children it did not list.
    fn has_children_at_edge(&self) -> bool {
        self.has_children
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use surya_proto::files::FileEvent;

    fn entry(path: &str, kind: FileKind, edge: bool) -> FileEntry {
        FileEntry {
            path: path.into(),
            kind,
            status: String::new(),
            has_children: edge,
            size: 0,
        }
    }

    fn tree(path: &str, entries: Vec<FileEntry>) -> FileTree {
        FileTree {
            path: path.into(),
            entries,
            truncated: false,
        }
    }

    fn labels(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|r| format!("{}{}", "  ".repeat(r.depth), r.label))
            .collect()
    }

    #[test]
    fn expand_lists_lazily_and_folds_single_child_chains() {
        let mut m = TreeModel::default();
        m.apply_tree(&tree(
            "",
            vec![
                entry("README.md", FileKind::File, false),
                entry("src", FileKind::Dir, true),
            ],
        ));
        assert_eq!(labels(&m.rows()), vec!["README.md", "src"]);
        assert!(m.toggle("src"), "first expand needs a listing");
        m.apply_tree(&tree(
            "src",
            vec![
                entry("src/main.rs", FileKind::File, false),
                entry("src/deep", FileKind::Dir, true),
            ],
        ));
        assert_eq!(
            labels(&m.rows()),
            vec!["README.md", "src", "  deep", "  main.rs"]
        );
        // deep has one child dir: after listing, the chain folds.
        assert!(m.toggle("src/deep"));
        m.apply_tree(&tree(
            "src/deep",
            vec![entry("src/deep/only", FileKind::Dir, true)],
        ));
        assert_eq!(
            labels(&m.rows()),
            vec!["README.md", "src", "  deep/only", "  main.rs"]
        );
        assert!(!m.toggle("src"), "collapse asks for nothing");
        assert_eq!(labels(&m.rows()), vec!["README.md", "src"]);
        assert!(
            !m.toggle("src"),
            "re-expand of a loaded dir needs no listing"
        );
        println!("expand: toggles=4 listings_asked=2");
    }

    #[test]
    fn watch_batches_apply_in_place() {
        let mut m = TreeModel::default();
        m.apply_tree(&tree(
            "",
            vec![
                entry("a.txt", FileKind::File, false),
                entry("src", FileKind::Dir, true),
            ],
        ));
        m.toggle("src");
        m.apply_tree(&tree(
            "src",
            vec![entry("src/main.rs", FileKind::File, false)],
        ));
        m.selected = Some("a.txt".into());
        let ev = |kind, path: &str, from: Option<&str>| FileEvent {
            kind,
            path: path.into(),
            from: from.map(Into::into),
        };
        let batch = FileWatchBatch {
            seq: 1,
            events: vec![
                ev(FileEventKind::Create, "b.txt", None),
                ev(FileEventKind::Delete, "a.txt", None),
                ev(FileEventKind::Rename, "src/lib.rs", Some("src/main.rs")),
                ev(FileEventKind::Modify, "src/lib.rs", None),
                ev(FileEventKind::Create, "unloaded/x.rs", None),
            ],
            truncated: false,
        };
        let refresh = m.apply_watch(&batch);
        println!("watch: events={} applied={}", m.events_seen, m.applied);
        assert_eq!((m.events_seen, m.applied), (5, 5));
        assert_eq!(labels(&m.rows()), vec!["b.txt", "src", "  lib.rs"]);
        assert_eq!(m.selected, None, "the deleted selection is dropped");
        assert_eq!(refresh, vec!["".to_string(), "src".to_string()]);
        // A truncated batch re-lists every loaded directory.
        let refresh = m.apply_watch(&FileWatchBatch {
            seq: 2,
            events: vec![],
            truncated: true,
        });
        let mut refresh = refresh;
        refresh.sort();
        assert_eq!(refresh, vec!["".to_string(), "src".to_string()]);
    }

    #[test]
    fn relist_replaces_only_the_listed_levels() {
        let mut m = TreeModel::default();
        m.apply_tree(&tree("", vec![entry("src", FileKind::Dir, true)]));
        m.toggle("src");
        m.apply_tree(&tree(
            "src",
            vec![
                entry("src/a.rs", FileKind::File, false),
                entry("src/deep", FileKind::Dir, true),
            ],
        ));
        m.toggle("src/deep");
        m.apply_tree(&tree(
            "src/deep",
            vec![entry("src/deep/x.rs", FileKind::File, false)],
        ));
        // Re-list `src` at depth 1: a.rs is gone, deep still there, and the
        // deeper level we already know survives untouched.
        m.apply_tree(&tree("src", vec![entry("src/deep", FileKind::Dir, true)]));
        assert!(m.entry("src/a.rs").is_none());
        assert!(m.entry("src/deep/x.rs").is_some());
    }

    #[test]
    fn selection_moves_and_clamps() {
        let mut m = TreeModel::default();
        m.apply_tree(&tree(
            "",
            vec![
                entry("a", FileKind::File, false),
                entry("b", FileKind::File, false),
            ],
        ));
        let rows = m.rows();
        assert_eq!(m.move_selection(&rows, 1).as_deref(), Some("a"));
        assert_eq!(m.move_selection(&rows, 1).as_deref(), Some("b"));
        assert_eq!(m.move_selection(&rows, 5).as_deref(), Some("b"));
        assert_eq!(m.move_selection(&rows, -1).as_deref(), Some("a"));
        assert_eq!(m.move_selection(&rows, -9).as_deref(), Some("a"));
    }
}
