//! Task board — engine side. One board per space; the user edits it in the
//! app, agents read and update it through the `surya-mcp` task tools. Rows
//! live in the registry (`zeron_doc::registry::tasks`), so every device and
//! every viewport on one engine sees the same board.
//!
//! This module owns the [`WorkspaceHost`] task API and the `WatchTasks`
//! stream shape. The `Mutate` op variants stay in `rpc.rs` (the tagged enum
//! must be one type); their bodies call in here.

use chrono::Utc;
use futures::StreamExt;
use futures::stream::BoxStream;
use serde::Deserialize;
use tokio::sync::watch;

use zeron_doc::TaskPatch;
use zeron_proto::{Task, TaskStatus};

use crate::EngineError;
use crate::workspace_host::WorkspaceHost;

/// `Mutate {op: "createTask"}` params. `task_id` is client-minted (the UI and
/// the MCP tool both mint UUIDs) so an optimistic row never flickers; `rank`
/// defaults to "bottom of the board".
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskParams {
    pub task_id: String,
    pub space_id: String,
    pub title: String,
    #[serde(default)]
    pub status: Option<TaskStatus>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub links: Option<Vec<String>>,
    #[serde(default)]
    pub rank: Option<f64>,
}

/// `Mutate {op: "updateTask"}` params — every field optional; absent leaves
/// the field alone. Clearing `owner` / `notes` is an explicit JSON `null`
/// (`Option<Option<_>>` keeps "absent" and "null" apart).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskParams {
    pub task_id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub status: Option<TaskStatus>,
    #[serde(default, deserialize_with = "double_option")]
    pub owner: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub notes: Option<Option<String>>,
    #[serde(default)]
    pub links: Option<Vec<String>>,
}

/// serde: absent → `None`, `null` → `Some(None)`, value → `Some(Some(v))`.
fn double_option<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer).map(Some)
}

/// `WatchTasks` params: `{spaceId?}` — one board, or every board when absent.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchTasksParams {
    #[serde(default)]
    pub space_id: Option<String>,
}

impl WorkspaceHost {
    /// Create a task. Idempotent by id: an existing live row is left alone
    /// (the second client's optimistic create must not clobber edits).
    pub fn create_task(&self, params: CreateTaskParams) -> Result<Task, EngineError> {
        if params.title.trim().is_empty() {
            return Err(EngineError::Other("task title must not be empty".into()));
        }
        if self.space(&params.space_id)?.is_none() {
            return Err(EngineError::Other(format!(
                "unknown space {}",
                params.space_id
            )));
        }
        if let Some(existing) = self.read(|doc| doc.task(&params.task_id))? {
            return Ok(existing);
        }
        let now = Utc::now();
        let task = self.mutate(|doc| -> Result<Task, EngineError> {
            let rank = match params.rank {
                Some(rank) if rank.is_finite() => rank,
                Some(rank) => {
                    return Err(EngineError::Other(format!(
                        "task rank must be finite, got {rank}"
                    )));
                }
                None => doc.next_task_rank(&params.space_id)?,
            };
            let task = Task {
                id: params.task_id,
                space_id: params.space_id,
                title: params.title.trim().to_string(),
                status: params.status.unwrap_or_default(),
                owner: params.owner,
                notes: params.notes,
                links: params.links.unwrap_or_default(),
                rank,
                created_at: now,
                updated_at: now,
            };
            doc.upsert_task(&task)?;
            Ok(task)
        })?;
        Ok(task)
    }

    /// LWW patch; `false` when no such task.
    pub fn update_task(&self, params: UpdateTaskParams) -> Result<bool, EngineError> {
        if let Some(title) = &params.title
            && title.trim().is_empty()
        {
            return Err(EngineError::Other("task title must not be empty".into()));
        }
        let patch = TaskPatch {
            title: params.title.map(|t| t.trim().to_string()),
            status: params.status,
            owner: params.owner,
            notes: params.notes,
            links: params.links,
        };
        Ok(self.mutate(|doc| doc.update_task(&params.task_id, &patch, Utc::now()))?)
    }

    /// Set the board position (float rank); `false` when no such task.
    pub fn reorder_task(&self, task_id: &str, rank: f64) -> Result<bool, EngineError> {
        Ok(self.mutate(|doc| doc.reorder_task(task_id, rank))?)
    }

    /// Tombstone; `false` when no live row existed.
    pub fn delete_task(&self, task_id: &str) -> Result<bool, EngineError> {
        Ok(self.mutate(|doc| doc.delete_task(task_id))?)
    }

    pub fn task(&self, task_id: &str) -> Result<Option<Task>, EngineError> {
        Ok(self.read(|doc| doc.task(task_id))?)
    }

    /// Every task, or one board's, in board order.
    pub fn read_tasks(&self, space_id: Option<&str>) -> Result<Vec<Task>, EngineError> {
        Ok(self.read(|doc| match space_id {
            Some(space_id) => doc.read_space_tasks(space_id),
            None => doc.read_tasks(),
        })?)
    }
}

/// `WatchTasks` stream: the current board first, then a fresh snapshot on
/// every registry change that touched it. Filtered per space so a board
/// viewer is not woken by another board's traffic.
pub fn watch_tasks_stream(
    rx: watch::Receiver<Vec<Task>>,
    space_id: Option<String>,
) -> BoxStream<'static, serde_json::Value> {
    let select = move |tasks: &[Task]| -> Vec<Task> {
        match &space_id {
            Some(space_id) => tasks
                .iter()
                .filter(|t| &t.space_id == space_id)
                .cloned()
                .collect(),
            None => tasks.to_vec(),
        }
    };
    futures::stream::unfold((rx, None::<Vec<Task>>), move |(mut rx, last)| {
        let select = select.clone();
        async move {
            loop {
                if last.is_some() {
                    rx.changed().await.ok()?;
                }
                let next = select(&rx.borrow_and_update());
                // Skip republishes that did not touch this board.
                if last.as_ref() == Some(&next) {
                    continue;
                }
                let value = serde_json::to_value(&next).ok()?;
                return Some((value, (rx, Some(next))));
            }
        }
    })
    .boxed()
}
