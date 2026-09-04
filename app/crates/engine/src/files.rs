//! File RPCs on the engine: a jailed tree, read, and optimistic-concurrency
//! write against one checkout root. This is haktui's resident file agent
//! (`haktui_files`, design.md D8/D13) folded into the engine; the wire types
//! and the mapping table live in `zeron_proto::files`. The watcher is
//! [`crate::files_watch`].
//!
//! Every function here takes a [`Jail`] and a relative path. The jail is the
//! whole security story: a path is joined to the root lexically (no `..`, no
//! absolute segment), then the nearest existing ancestor is canonicalized and
//! must still sit under the canonical root, so a symlink cannot lead out.
//! Same rule as `uploads.rs`, applied to a checkout instead of the cache.
//!
//! All functions are blocking; callers run them on the blocking pool.

use std::path::{Component, Path, PathBuf};
use std::process::Command;

use sha2::{Digest as _, Sha256};
use zeron_proto::files::{FileEntry, FileKind, FileRead, FileTree, FileWrite, LineRange};

use crate::EngineError;

/// Largest file `FilesRead` sends back.
pub const MAX_READ_BYTES: u64 = 8 * 1024 * 1024;
/// Most entries one `FilesTree` answer carries (haktui: 20k, its column was
/// 198px wide; this is per lazy request, so lower).
pub const MAX_TREE_ENTRIES: usize = 10_000;
/// Deepest `depth` a tree request may ask for.
pub const MAX_TREE_DEPTH: u32 = 8;
/// Bytes sniffed for a NUL when deciding text vs binary.
const BINARY_SNIFF: usize = 8 * 1024;

fn other(msg: impl Into<String>) -> EngineError {
    EngineError::Other(msg.into())
}

/// A canonical checkout root and the rule for paths under it.
#[derive(Debug, Clone)]
pub struct Jail {
    root: PathBuf,
}

impl Jail {
    /// `root` must exist and be a directory; it is canonicalized once.
    pub fn new(root: &Path) -> Result<Self, EngineError> {
        let root = std::fs::canonicalize(root)?;
        if !root.is_dir() {
            return Err(other("checkout root is not a directory"));
        }
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Resolve a relative wire path to an absolute path inside the root, or
    /// refuse. The target itself may not exist yet (writes create files), so
    /// the check canonicalizes the nearest existing ancestor.
    pub fn resolve(&self, rel: &str) -> Result<PathBuf, EngineError> {
        let outside = || other("path is outside the checkout");
        let rel = rel.trim_start_matches('/');
        let mut joined = self.root.clone();
        for component in Path::new(rel).components() {
            match component {
                Component::Normal(part) => joined.push(part),
                Component::CurDir => {}
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(outside());
                }
            }
        }
        // Walk up to the first ancestor that exists and canonicalize it; the
        // remaining tail is lexical and already free of `..`.
        let mut existing = joined.as_path();
        let mut tail = Vec::new();
        while !existing.exists() {
            let Some(parent) = existing.parent() else {
                return Err(outside());
            };
            if let Some(name) = existing.file_name() {
                tail.push(name.to_os_string());
            }
            existing = parent;
        }
        let mut real = std::fs::canonicalize(existing).map_err(|_| outside())?;
        if !real.starts_with(&self.root) {
            return Err(outside());
        }
        for part in tail.into_iter().rev() {
            real.push(part);
        }
        Ok(real)
    }

    /// The wire form of an absolute path under the root.
    pub fn relative(&self, abs: &Path) -> Option<String> {
        abs.strip_prefix(&self.root)
            .ok()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
    }
}

/// sha256 of `bytes`, lowercase hex.
pub fn content_hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

// ---------------------------------------------------------------------------
// Tree
// ---------------------------------------------------------------------------

/// The entries under `path`, `depth` levels deep, honoring `.gitignore`,
/// `.ignore`, global excludes and `.git/info/exclude` the way the composer's
/// file search does. `.git` itself never appears.
pub fn tree(jail: &Jail, path: &str, depth: u32) -> Result<FileTree, EngineError> {
    let depth = depth.clamp(1, MAX_TREE_DEPTH);
    let dir = jail.resolve(path)?;
    if !dir.is_dir() {
        return Err(other("tree path is not a directory"));
    }
    let statuses = git_status(jail, &dir);
    let mut entries = Vec::new();
    let mut truncated = false;
    let walker = ignore::WalkBuilder::new(&dir)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .max_depth(Some(depth as usize))
        .sort_by_file_path(|a, b| a.cmp(b))
        .filter_entry(|entry| entry.depth() == 0 || entry.file_name() != ".git")
        .build();
    for entry in walker {
        let Ok(entry) = entry else { continue };
        if entry.depth() == 0 {
            continue;
        }
        if entries.len() >= MAX_TREE_ENTRIES {
            truncated = true;
            break;
        }
        let abs = entry.path();
        let Some(rel) = jail.relative(abs) else {
            continue;
        };
        let file_type = entry.file_type();
        let kind = if file_type.is_some_and(|t| t.is_symlink()) {
            FileKind::Symlink
        } else if file_type.is_some_and(|t| t.is_dir()) {
            FileKind::Dir
        } else {
            FileKind::File
        };
        let size = if kind == FileKind::File {
            entry.metadata().map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };
        let has_children = kind == FileKind::Dir
            && entry.depth() as u32 == depth
            && std::fs::read_dir(abs)
                .map(|mut d| d.next().is_some())
                .unwrap_or(false);
        let status = match kind {
            FileKind::Dir => strongest_under(&statuses, &rel),
            _ => statuses
                .iter()
                .find(|(p, _)| p == &rel)
                .map(|(_, s)| s.clone())
                .unwrap_or_default(),
        };
        entries.push(FileEntry {
            path: rel,
            kind,
            status,
            has_children,
            size,
        });
    }
    Ok(FileTree {
        path: jail.relative(&dir).unwrap_or_default(),
        entries,
        truncated,
    })
}

fn rank(s: &str) -> u8 {
    match s {
        "D" => 4,
        "M" => 3,
        "A" => 2,
        "?" => 1,
        _ => 0,
    }
}

fn strongest_under(statuses: &[(String, String)], dir_rel: &str) -> String {
    let prefix = format!("{dir_rel}/");
    statuses
        .iter()
        .filter(|(p, _)| p.starts_with(&prefix))
        .map(|(_, s)| s.as_str())
        .max_by_key(|s| rank(s))
        .unwrap_or("")
        .to_string()
}

/// `git status --porcelain -z` for the subtree at `dir`, as (path relative to
/// the jail root, one-letter status). Empty outside a repository.
pub fn git_status(jail: &Jail, dir: &Path) -> Vec<(String, String)> {
    let Ok(out) = Command::new("git")
        .arg("-C")
        .arg(jail.root())
        .args(["status", "--porcelain", "-z", "--untracked-files=all", "--"])
        .arg(dir)
        .output()
    else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    // Paths come back relative to the repository top, which may sit above the
    // jail root (a space inside a bigger repo); re-base them.
    let top = Command::new("git")
        .arg("-C")
        .arg(jail.root())
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim()));
    let prefix = top
        .and_then(|top| jail.root().strip_prefix(top).ok().map(Path::to_path_buf))
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    let mut v = Vec::new();
    let mut fields = out.stdout.split(|b| *b == 0).filter(|s| s.len() >= 3);
    while let Some(f) = fields.next() {
        let xy = &f[..2];
        let path = String::from_utf8_lossy(&f[3..]).into_owned();
        if xy[0] == b'R' || xy[0] == b'C' {
            let _ = fields.next(); // the old name
        }
        let s = match (xy[0], xy[1]) {
            (b'?', b'?') => "?",
            (b'D', _) | (_, b'D') => "D",
            (b'A', _) | (b'R', _) | (b'C', _) => "A",
            (b'M', _) | (_, b'M') => "M",
            _ => "",
        };
        if s.is_empty() {
            continue;
        }
        let rel = if prefix.is_empty() {
            path
        } else if let Some(r) = path.strip_prefix(&format!("{prefix}/")) {
            r.to_string()
        } else {
            continue;
        };
        v.push((rel, s.to_string()));
    }
    v
}

// ---------------------------------------------------------------------------
// Read
// ---------------------------------------------------------------------------

pub fn read(jail: &Jail, path: &str, range: Option<LineRange>) -> Result<FileRead, EngineError> {
    let abs = jail.resolve(path)?;
    let meta = std::fs::metadata(&abs)?;
    if !meta.is_file() {
        return Err(other("not a file"));
    }
    if meta.len() > MAX_READ_BYTES {
        return Ok(FileRead::TooLarge {
            size: meta.len(),
            max: MAX_READ_BYTES,
        });
    }
    let bytes = std::fs::read(&abs)?;
    let size = bytes.len() as u64;
    if bytes[..bytes.len().min(BINARY_SNIFF)].contains(&0) {
        return Ok(FileRead::Binary { size });
    }
    let Ok(text) = String::from_utf8(bytes) else {
        return Ok(FileRead::Binary { size });
    };
    let hash = content_hash(text.as_bytes());
    let lines = text.lines().count() as u64;
    let text = match range {
        None => text,
        Some(LineRange { start, end }) => text
            .split_inclusive('\n')
            .skip(start as usize)
            .take(end.saturating_sub(start) as usize)
            .collect(),
    };
    Ok(FileRead::Text {
        text,
        hash,
        size,
        lines,
    })
}

// ---------------------------------------------------------------------------
// Write
// ---------------------------------------------------------------------------

/// haktui D13: re-derive the hash from disk and compare in the same operation
/// as the write; refuse on mismatch and hand back what is there now; write a
/// temp file beside the target and rename it into place.
pub fn write(
    jail: &Jail,
    path: &str,
    content: &str,
    expected_hash: Option<&str>,
) -> Result<FileWrite, EngineError> {
    let abs = jail.resolve(path)?;
    let current = match std::fs::read(&abs) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    let current_hash = current.as_deref().map(content_hash);
    match (expected_hash, &current) {
        (None, Some(bytes)) => {
            return Ok(FileWrite::Refused {
                reason: "a file already exists at that path".into(),
                text: String::from_utf8(bytes.clone()).ok(),
                hash: current_hash,
            });
        }
        (Some(_), None) => {
            return Ok(FileWrite::Refused {
                reason: "the file is gone".into(),
                text: None,
                hash: None,
            });
        }
        (Some(expected), Some(bytes)) if current_hash.as_deref() != Some(expected) => {
            return Ok(FileWrite::Refused {
                reason: "changed on disk since it was read".into(),
                text: String::from_utf8(bytes.clone()).ok(),
                hash: current_hash,
            });
        }
        _ => {}
    }
    // Keep the file's line endings: CRLF on disk and LF in the buffer means
    // the editor normalized; put them back.
    let uses_crlf = current
        .as_deref()
        .is_some_and(|b| b.windows(2).any(|w| w == b"\r\n"));
    let bytes: Vec<u8> = if uses_crlf && !content.contains("\r\n") {
        content.replace('\n', "\r\n").into_bytes()
    } else {
        content.as_bytes().to_vec()
    };
    let dir = abs.parent().ok_or_else(|| other("path has no parent"))?;
    if !dir.exists() {
        return Err(other("parent directory does not exist"));
    }
    let tmp = dir.join(format!(
        ".{}.zeron-{}",
        abs.file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default(),
        std::process::id()
    ));
    std::fs::write(&tmp, &bytes)?;
    #[cfg(unix)]
    if let Ok(meta) = std::fs::metadata(&abs) {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(
            &tmp,
            std::fs::Permissions::from_mode(meta.permissions().mode()),
        );
    }
    if let Err(e) = std::fs::rename(&tmp, &abs) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.into());
    }
    Ok(FileWrite::Saved {
        hash: content_hash(&bytes),
        size: bytes.len() as u64,
    })
}
