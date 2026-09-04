//! Task board rows in the registry (`kind = "tasks"`, one row per task, keyed
//! by task id). Every field is an independent LWW write, so an agent stamping
//! `status` and the user editing `notes` on another device both land.
//!
//! Writer discipline: any device (or agent through the engine) may create,
//! update, reorder, or delete a task. Nothing here is device-owned.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{Value, json};

use zeron_proto::{Task, TaskStatus, sort_tasks};

use super::{KIND_TASKS, OpKind, RegistryDoc, fields, opt_str, row_to};
use crate::schema::DocError;

/// Field-level patch for [`RegistryDoc::update_task`]: `None` leaves the
/// field alone, `Some(None)` clears it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TaskPatch {
    pub title: Option<String>,
    pub status: Option<TaskStatus>,
    pub owner: Option<Option<String>>,
    pub notes: Option<Option<String>>,
    pub links: Option<Vec<String>>,
}

impl TaskPatch {
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.status.is_none()
            && self.owner.is_none()
            && self.notes.is_none()
            && self.links.is_none()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawTask {
    id: String,
    space_id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    status: TaskStatus,
    #[serde(default)]
    owner: Option<String>,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    links: Vec<String>,
    #[serde(default)]
    rank: f64,
    #[serde(default)]
    created_at: i64,
    #[serde(default)]
    updated_at: i64,
}

impl From<RawTask> for Task {
    fn from(raw: RawTask) -> Self {
        Task {
            id: raw.id,
            space_id: raw.space_id,
            title: raw.title,
            status: raw.status,
            owner: raw.owner,
            notes: raw.notes,
            links: raw.links,
            rank: raw.rank,
            created_at: ms_to_dt(raw.created_at),
            updated_at: ms_to_dt(raw.updated_at),
        }
    }
}

fn ms_to_dt(ms: i64) -> DateTime<Utc> {
    DateTime::from_timestamp_millis(ms).unwrap_or(DateTime::UNIX_EPOCH)
}

impl RegistryDoc {
    /// Create (or fully rewrite) a task row. The caller mints the id and the
    /// rank; see [`RegistryDoc::next_task_rank`] for "append at the bottom".
    pub fn upsert_task(&mut self, task: &Task) -> Result<(), DocError> {
        let set = fields([
            ("id", json!(task.id)),
            ("spaceId", json!(task.space_id)),
            ("title", json!(task.title)),
            ("status", json!(task.status)),
            ("owner", opt_str(task.owner.as_deref())),
            ("notes", opt_str(task.notes.as_deref())),
            ("links", json!(task.links)),
            ("rank", json!(task.rank)),
            ("createdAt", json!(task.created_at.timestamp_millis())),
            ("updatedAt", json!(task.updated_at.timestamp_millis())),
        ]);
        self.write(KIND_TASKS, &task.id.clone(), OpKind::Upsert, set);
        Ok(())
    }

    /// One task (overlay view), `None` when missing or deleted.
    pub fn task(&self, task_id: &str) -> Result<Option<Task>, DocError> {
        Ok(self
            .overlay_row(KIND_TASKS, task_id)
            .and_then(|row| row_to::<RawTask>(&row))
            .map(Task::from))
    }

    /// Every live task, board order (rank, created, id).
    pub fn read_tasks(&self) -> Result<Vec<Task>, DocError> {
        let mut tasks: Vec<Task> = self
            .read_kind::<RawTask>(KIND_TASKS)
            .into_iter()
            .map(Task::from)
            .collect();
        sort_tasks(&mut tasks);
        Ok(tasks)
    }

    /// The tasks of one board, board order.
    pub fn read_space_tasks(&self, space_id: &str) -> Result<Vec<Task>, DocError> {
        let mut tasks = self.read_tasks()?;
        tasks.retain(|t| t.space_id == space_id);
        Ok(tasks)
    }

    /// Rank that appends a task at the bottom of `space_id`'s board.
    pub fn next_task_rank(&self, space_id: &str) -> Result<f64, DocError> {
        Ok(self
            .read_space_tasks(space_id)?
            .last()
            .map_or(1.0, |t| t.rank + 1.0))
    }

    /// LWW patch of the editable fields plus `updatedAt`. Never invents a row:
    /// `false` when no such task.
    pub fn update_task(
        &mut self,
        task_id: &str,
        patch: &TaskPatch,
        at: DateTime<Utc>,
    ) -> Result<bool, DocError> {
        if !self.row_exists(KIND_TASKS, task_id) {
            return Ok(false);
        }
        if patch.is_empty() {
            return Ok(true);
        }
        let mut set: BTreeMap<String, Value> = BTreeMap::new();
        if let Some(title) = &patch.title {
            set.insert("title".into(), json!(title));
        }
        if let Some(status) = patch.status {
            set.insert("status".into(), json!(status));
        }
        if let Some(owner) = &patch.owner {
            set.insert("owner".into(), opt_str(owner.as_deref()));
        }
        if let Some(notes) = &patch.notes {
            set.insert("notes".into(), opt_str(notes.as_deref()));
        }
        if let Some(links) = &patch.links {
            set.insert("links".into(), json!(links));
        }
        set.insert("updatedAt".into(), json!(at.timestamp_millis()));
        self.write(KIND_TASKS, task_id, OpKind::Update, set);
        Ok(true)
    }

    /// Set the board position. One clocked field write; the read side sorts.
    /// `false` when no such task.
    pub fn reorder_task(&mut self, task_id: &str, rank: f64) -> Result<bool, DocError> {
        if !rank.is_finite() {
            return Err(DocError::Schema(format!(
                "task rank must be finite, got {rank}"
            )));
        }
        if !self.row_exists(KIND_TASKS, task_id) {
            return Ok(false);
        }
        self.write(
            KIND_TASKS,
            task_id,
            OpKind::Update,
            fields([("rank", json!(rank))]),
        );
        Ok(true)
    }

    /// Tombstone one task. `false` when no live row existed (the tombstone is
    /// still written so a late create cannot revive it).
    pub fn delete_task(&mut self, task_id: &str) -> Result<bool, DocError> {
        let existed = self.row_exists(KIND_TASKS, task_id);
        self.delete_row_ops(&[(KIND_TASKS, task_id)]);
        Ok(existed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(ms: i64) -> DateTime<Utc> {
        ms_to_dt(ms)
    }

    fn task(id: &str, rank: f64) -> Task {
        Task {
            id: id.into(),
            space_id: "sp-1".into(),
            title: format!("Task {id}"),
            status: TaskStatus::Queued,
            owner: None,
            notes: None,
            links: vec![],
            rank,
            created_at: at(1_000),
            updated_at: at(1_000),
        }
    }

    #[test]
    fn create_read_update_delete_round_trip() {
        let mut doc = RegistryDoc::new("dev-a");
        doc.upsert_task(&task("t-1", 1.0)).unwrap();
        let read = doc.task("t-1").unwrap().expect("row present");
        assert_eq!(read.title, "Task t-1");
        assert_eq!(read.status, TaskStatus::Queued);

        let patch = TaskPatch {
            status: Some(TaskStatus::Running),
            owner: Some(Some("agent-7".into())),
            notes: Some(Some("on it".into())),
            links: Some(vec!["https://github.com/x/y/pull/1".into()]),
            ..TaskPatch::default()
        };
        assert!(doc.update_task("t-1", &patch, at(2_000)).unwrap());
        let read = doc.task("t-1").unwrap().unwrap();
        assert_eq!(read.status, TaskStatus::Running);
        assert_eq!(read.owner.as_deref(), Some("agent-7"));
        assert_eq!(read.notes.as_deref(), Some("on it"));
        assert_eq!(read.links.len(), 1);
        assert_eq!(read.updated_at, at(2_000));
        assert_eq!(read.created_at, at(1_000), "created never moves");

        // Clearing a field is an explicit Some(None).
        let clear = TaskPatch {
            owner: Some(None),
            ..TaskPatch::default()
        };
        assert!(doc.update_task("t-1", &clear, at(3_000)).unwrap());
        assert_eq!(doc.task("t-1").unwrap().unwrap().owner, None);

        assert!(!doc.update_task("nope", &patch, at(4_000)).unwrap());
        assert!(doc.delete_task("t-1").unwrap());
        assert!(doc.task("t-1").unwrap().is_none());
        assert!(!doc.delete_task("t-1").unwrap());
    }

    #[test]
    fn reorder_changes_board_order_and_rank_appends() {
        let mut doc = RegistryDoc::new("dev-a");
        for (id, rank) in [("t-1", 1.0), ("t-2", 2.0), ("t-3", 3.0)] {
            doc.upsert_task(&task(id, rank)).unwrap();
        }
        assert_eq!(doc.next_task_rank("sp-1").unwrap(), 4.0);
        assert_eq!(doc.next_task_rank("sp-empty").unwrap(), 1.0);

        // Move t-3 between t-1 and t-2 with one write.
        assert!(doc.reorder_task("t-3", 1.5).unwrap());
        let ids: Vec<String> = doc
            .read_space_tasks("sp-1")
            .unwrap()
            .into_iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(ids, ["t-1", "t-3", "t-2"]);
        assert!(!doc.reorder_task("nope", 9.0).unwrap());
        assert!(doc.reorder_task("t-1", f64::NAN).is_err());
    }

    #[test]
    fn rows_survive_a_snapshot_round_trip() {
        let mut doc = RegistryDoc::new("dev-a");
        doc.upsert_task(&task("t-1", 2.0)).unwrap();
        doc.upsert_task(&task("t-2", 1.0)).unwrap();
        let bytes = doc.to_bytes().unwrap();
        let reopened = RegistryDoc::from_bytes(&bytes, "dev-a").unwrap();
        let ids: Vec<String> = reopened
            .read_tasks()
            .unwrap()
            .into_iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(ids, ["t-2", "t-1"]);
    }

    #[test]
    fn tasks_in_other_spaces_are_filtered() {
        let mut doc = RegistryDoc::new("dev-a");
        doc.upsert_task(&task("t-1", 1.0)).unwrap();
        let mut other = task("t-9", 1.0);
        other.space_id = "sp-2".into();
        doc.upsert_task(&other).unwrap();
        assert_eq!(doc.read_space_tasks("sp-1").unwrap().len(), 1);
        assert_eq!(doc.read_tasks().unwrap().len(), 2);
    }
}
