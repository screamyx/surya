//! Task board rows — one board per space, tracked by the user in the app and
//! read/updated by agents through the `surya-mcp` task tools.
//!
//! Wire shape follows the mockup's Task type (`mockup/src/data.ts`): id,
//! title, status, owner, workspace, notes, links. `rank` is the board order
//! (a float so a reorder writes ONE row: the moved task takes the midpoint
//! of its new neighbours).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    #[default]
    Queued,
    Running,
    Done,
    Blocked,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Done => "done",
            Self::Blocked => "blocked",
        }
    }

    /// Parse the wire string; `None` for anything outside the four states.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "queued" => Some(Self::Queued),
            "running" => Some(Self::Running),
            "done" => Some(Self::Done),
            "blocked" => Some(Self::Blocked),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    /// The board the task belongs to (a space id).
    pub space_id: String,
    pub title: String,
    #[serde(default)]
    pub status: TaskStatus,
    /// Agent id, or the literal `user`; absent = unowned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// PR / issue / doc URLs.
    #[serde(default)]
    pub links: Vec<String>,
    /// Board order within the space; ascending. Ties break on `created_at`, then id.
    #[serde(default)]
    pub rank: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Board order: rank, then creation time, then id — total and stable.
pub fn sort_tasks(tasks: &mut [Task]) {
    tasks.sort_by(|a, b| {
        a.rank
            .total_cmp(&b.rank)
            .then_with(|| a.created_at.cmp(&b.created_at))
            .then_with(|| a.id.cmp(&b.id))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_round_trips_lowercase() {
        for status in [
            TaskStatus::Queued,
            TaskStatus::Running,
            TaskStatus::Done,
            TaskStatus::Blocked,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, format!("\"{}\"", status.as_str()));
            assert_eq!(TaskStatus::parse(status.as_str()), Some(status));
        }
        assert_eq!(TaskStatus::parse("paused"), None);
    }

    #[test]
    fn sort_is_by_rank_then_created_then_id() {
        let at = |ms: i64| DateTime::from_timestamp_millis(ms).unwrap();
        let mk = |id: &str, rank: f64, ms: i64| Task {
            id: id.into(),
            space_id: "sp".into(),
            title: id.into(),
            status: TaskStatus::Queued,
            owner: None,
            notes: None,
            links: vec![],
            rank,
            created_at: at(ms),
            updated_at: at(ms),
        };
        let mut tasks = vec![mk("c", 2.0, 1), mk("b", 1.0, 2), mk("a", 1.0, 1)];
        sort_tasks(&mut tasks);
        let ids: Vec<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["a", "b", "c"]);
    }
}
