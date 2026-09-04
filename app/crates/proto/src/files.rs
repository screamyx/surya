//! Wire types for the engine's file RPCs (`FilesTree`, `FilesWatch`,
//! `FilesRead`, `FilesWrite`, `FilesSearch`): the tree and editor data path
//! that haktui served from a separate resident file agent. In surya the
//! engine sits on the machine with the files, so the agent's protocol becomes
//! engine RPCs jailed to one space's checkout root.
//!
//! Mapping from haktui's `haktui_files` protocol, for whoever ports the tree
//! and editor views:
//!
//! | haktui                          | surya                                   |
//! |---------------------------------|-----------------------------------------|
//! | `tree {root}` full, eager       | `FilesTree {spaceId, path, depth}` lazy |
//! | `Entry {path, kind, status}`    | [`FileEntry`] + `hasChildren`           |
//! | `read -> {text, token}`         | [`FileRead`] with `hash` (sha256)       |
//! | `read -> {binary, size}`        | [`FileRead::Binary`]                    |
//! | `save {path, text, token}`      | `FilesWrite {.., content, expectedHash}`|
//! | `saved:false, reason, text`     | [`FileWrite::Refused`]                  |
//! | `changed {paths, diff}` event   | [`FileWatchBatch`] of [`FileEvent`]s    |
//!
//! Every path on the wire is relative to the checkout root, forward slashes,
//! never absolute. The hash is a content hash only (no mtime), so a save that
//! rewrites identical bytes still matches.

use serde::{Deserialize, Serialize};

/// `FilesTree` params. `path` is a directory relative to the checkout root
/// (`""` for the root itself); `depth` is how many directory levels to
/// expand below it (1 = just its children).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileTreeParams {
    pub space_id: String,
    #[serde(default)]
    pub path: String,
    #[serde(default = "default_depth")]
    pub depth: u32,
}

fn default_depth() -> u32 {
    1
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    File,
    Dir,
    Symlink,
}

/// One entry of a tree answer. `status` is git's short status for the path:
/// `"M"`, `"A"`, `"?"`, `"D"`, or `""` when clean. A directory carries the
/// strongest status of anything under it (haktui's rule, kept).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub path: String,
    pub kind: FileKind,
    #[serde(default)]
    pub status: String,
    /// For a directory at the depth limit: whether it has anything under
    /// it, so the tree can draw a disclosure arrow before expanding.
    #[serde(default)]
    pub has_children: bool,
    #[serde(default)]
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileTree {
    /// The directory listed, relative to the checkout root.
    pub path: String,
    /// Sorted by path; directories and files interleaved as `sort` would.
    pub entries: Vec<FileEntry>,
    /// Entry cap hit; the listing is incomplete.
    pub truncated: bool,
}

/// `FilesWatch` params.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileWatchParams {
    pub space_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileEventKind {
    Create,
    Modify,
    Delete,
    Rename,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEvent {
    pub kind: FileEventKind,
    /// The path the event is about; for a rename, the new name.
    pub path: String,
    /// For a rename whose old name the platform reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
}

/// One debounced burst of filesystem events under the checkout root.
/// `seq` climbs by one per batch on a stream. `truncated` says the burst
/// held more events than the cap, so the client should treat every open
/// file as possibly changed (haktui's `paths_truncated`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileWatchBatch {
    pub seq: u64,
    pub events: Vec<FileEvent>,
    #[serde(default)]
    pub truncated: bool,
}

/// `FilesRead` params. `range` is a line window, zero-based, end exclusive;
/// absent means the whole file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileReadParams {
    pub space_id: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<LineRange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineRange {
    pub start: u64,
    pub end: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FileRead {
    /// UTF-8 text. `hash` is sha256 of the whole file's bytes (also when a
    /// range was asked for), the value `FilesWrite` wants as `expectedHash`.
    Text {
        text: String,
        hash: String,
        size: u64,
        /// Total line count of the file, so a ranged read can page.
        lines: u64,
    },
    Binary { size: u64 },
    /// Larger than the read cap; not sent.
    TooLarge { size: u64, max: u64 },
}

/// `FilesWrite` params. `expectedHash` absent means "create; refuse if a
/// file already exists".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileWriteParams {
    pub space_id: String,
    pub path: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FileWrite {
    Saved { hash: String, size: u64 },
    /// The bytes on disk no longer match `expectedHash`. What is there now
    /// comes back so the editor can show the conflict.
    Refused {
        reason: String,
        text: Option<String>,
        hash: Option<String>,
    },
}

/// `FilesSearch` params: fuzzy match on relative path, the same matcher the
/// composer's `@file` mention uses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileNameSearchParams {
    pub space_id: String,
    pub query: String,
}
