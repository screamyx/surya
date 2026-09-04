//! `FilesWatch`: a debounced stream of filesystem events under one checkout
//! root. One recursive `notify` watcher per subscription; it lives exactly as
//! long as the stream, so dropping the subscription (the RPC cancel frame)
//! releases the inotify/FSEvents handles.
//!
//! Events are batched: the first event opens a window, the window closes
//! after [`DEBOUNCE`] of quiet, and the batch is deduplicated by (kind, path)
//! and capped at [`MAX_EVENTS_PER_BATCH`]. `.git/` and gitignored paths are
//! dropped before the cap counts them; haktui measured why (its bursts on a
//! busy tree were 200 directory paths and the one open file fell off the
//! end). The ignore rules come from the root `.gitignore` and
//! `.git/info/exclude`; nested `.gitignore` files are not consulted here.

use std::path::{Path, PathBuf};
use std::time::Duration;

use futures::StreamExt as _;
use futures::stream::BoxStream;
use notify::Watcher as _;
use tokio::sync::mpsc;
use zeron_proto::files::{FileEvent, FileEventKind, FileWatchBatch};

use crate::EngineError;
use crate::files::Jail;

/// Quiet time that closes a batch.
pub const DEBOUNCE: Duration = Duration::from_millis(150);
/// Most events one batch carries; past it the batch says `truncated`.
pub const MAX_EVENTS_PER_BATCH: usize = 500;

/// Start watching `jail`'s root. Returns the batch stream; the watcher is
/// torn down when the stream is dropped.
pub fn watch(jail: Jail) -> Result<BoxStream<'static, FileWatchBatch>, EngineError> {
    let (raw_tx, raw_rx) = mpsc::unbounded_channel::<notify::Event>();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if let Ok(event) = event {
            let _ = raw_tx.send(event);
        }
    })
    .map_err(|e| EngineError::Other(format!("file watcher: {e}")))?;
    watcher
        .watch(jail.root(), notify::RecursiveMode::Recursive)
        .map_err(|e| EngineError::Other(format!("file watcher: {e}")))?;
    let filter = IgnoreFilter::new(jail.root());
    let (batch_tx, batch_rx) = mpsc::channel::<FileWatchBatch>(64);
    tokio::spawn(pump(watcher, jail, filter, raw_rx, batch_tx));
    Ok(futures::stream::unfold(batch_rx, |mut rx| async move {
        rx.recv().await.map(|batch| (batch, rx))
    })
    .boxed())
}

/// Debounce loop. Owns the watcher; exits when the subscriber is gone.
async fn pump(
    _watcher: notify::RecommendedWatcher,
    jail: Jail,
    filter: IgnoreFilter,
    mut raw_rx: mpsc::UnboundedReceiver<notify::Event>,
    batch_tx: mpsc::Sender<FileWatchBatch>,
) {
    let mut seq = 0u64;
    loop {
        let first = tokio::select! {
            _ = batch_tx.closed() => return,
            event = raw_rx.recv() => match event {
                Some(event) => event,
                None => return,
            },
        };
        let mut raw = vec![first];
        loop {
            match tokio::time::timeout(DEBOUNCE, raw_rx.recv()).await {
                Ok(Some(event)) => raw.push(event),
                Ok(None) => return,
                Err(_) => break,
            }
        }
        let (events, truncated) = fold(&jail, &filter, raw);
        if events.is_empty() && !truncated {
            continue;
        }
        seq += 1;
        if batch_tx
            .send(FileWatchBatch {
                seq,
                events,
                truncated,
            })
            .await
            .is_err()
        {
            return;
        }
    }
}

/// Raw notify events to wire events: map kinds, jail-relative paths, drop
/// what the tree would not show, dedupe, cap.
fn fold(jail: &Jail, filter: &IgnoreFilter, raw: Vec<notify::Event>) -> (Vec<FileEvent>, bool) {
    use notify::event::{EventKind, ModifyKind, RenameMode};
    let mut events: Vec<FileEvent> = Vec::new();
    let mut truncated = false;
    let mut push = |events: &mut Vec<FileEvent>, ev: FileEvent| {
        if events
            .iter()
            .any(|e| e.kind == ev.kind && e.path == ev.path)
        {
            return;
        }
        if events.len() >= MAX_EVENTS_PER_BATCH {
            truncated = true;
            return;
        }
        events.push(ev);
    };
    for event in raw {
        let rel = |p: &PathBuf| -> Option<String> {
            let rel = jail.relative(p)?;
            (!rel.is_empty() && filter.keep(&rel, p.is_dir())).then_some(rel)
        };
        match event.kind {
            EventKind::Access(_) => continue,
            EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                let from = event.paths.first().and_then(rel);
                let Some(to) = event.paths.get(1).and_then(rel) else {
                    continue;
                };
                push(
                    &mut events,
                    FileEvent {
                        kind: FileEventKind::Rename,
                        path: to,
                        from,
                    },
                );
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                for p in event.paths.iter().filter_map(rel) {
                    push(
                        &mut events,
                        FileEvent {
                            kind: FileEventKind::Delete,
                            path: p,
                            from: None,
                        },
                    );
                }
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                for p in event.paths.iter().filter_map(rel) {
                    push(
                        &mut events,
                        FileEvent {
                            kind: FileEventKind::Create,
                            path: p,
                            from: None,
                        },
                    );
                }
            }
            EventKind::Modify(ModifyKind::Name(_)) => {
                for p in event.paths.iter().filter_map(rel) {
                    push(
                        &mut events,
                        FileEvent {
                            kind: FileEventKind::Rename,
                            path: p,
                            from: None,
                        },
                    );
                }
            }
            EventKind::Create(_) => {
                for p in event.paths.iter().filter_map(rel) {
                    push(
                        &mut events,
                        FileEvent {
                            kind: FileEventKind::Create,
                            path: p,
                            from: None,
                        },
                    );
                }
            }
            EventKind::Remove(_) => {
                for p in event.paths.iter().filter_map(rel) {
                    push(
                        &mut events,
                        FileEvent {
                            kind: FileEventKind::Delete,
                            path: p,
                            from: None,
                        },
                    );
                }
            }
            EventKind::Modify(_) | EventKind::Any | EventKind::Other => {
                for p in event.paths.iter().filter_map(rel) {
                    push(
                        &mut events,
                        FileEvent {
                            kind: FileEventKind::Modify,
                            path: p,
                            from: None,
                        },
                    );
                }
            }
        }
    }
    (events, truncated)
}

/// Root-level ignore rules for the watcher.
struct IgnoreFilter {
    gitignore: ignore::gitignore::Gitignore,
}

impl IgnoreFilter {
    fn new(root: &Path) -> Self {
        let mut builder = ignore::gitignore::GitignoreBuilder::new(root);
        let _ = builder.add(root.join(".gitignore"));
        let _ = builder.add(root.join(".git").join("info").join("exclude"));
        let gitignore = builder
            .build()
            .unwrap_or_else(|_| ignore::gitignore::Gitignore::empty());
        Self { gitignore }
    }

    fn keep(&self, rel: &str, is_dir: bool) -> bool {
        if rel == ".git" || rel.starts_with(".git/") {
            return false;
        }
        !self
            .gitignore
            .matched_path_or_any_parents(rel, is_dir)
            .is_ignore()
    }
}
