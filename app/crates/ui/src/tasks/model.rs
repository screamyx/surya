//! Board model behind the Tasks pane: pure data, no gpui. Groups the
//! `WatchTasks` snapshot into columns, tracks the keyboard selection, and
//! does the rank arithmetic for a drop (one row moves; a collision between
//! stale ranks renumbers the column).

use std::time::{Duration, Instant};

use surya_proto::{Task, TaskStatus, sort_tasks};

/// Column order on the board (brief: Queued, Running, Done, Blocked).
pub const COLUMNS: [TaskStatus; 4] = [
    TaskStatus::Queued,
    TaskStatus::Running,
    TaskStatus::Done,
    TaskStatus::Blocked,
];

pub fn column_label(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Queued => "Queued",
        TaskStatus::Running => "Running",
        TaskStatus::Done => "Done",
        TaskStatus::Blocked => "Blocked",
    }
}

/// Where a dragged card lands.
#[derive(Debug, Clone, PartialEq)]
pub enum DropTarget {
    /// Before this card (same or another column).
    Before(String),
    /// At the bottom of this column.
    End(TaskStatus),
}

/// The writes a drop turns into. `status` is `Some` when the card changed
/// column; `ranks` always names the moved card first and then any column
/// mates renumbered to clear a rank collision.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DropPlan {
    pub task_id: String,
    pub status: Option<TaskStatus>,
    pub ranks: Vec<(String, f64)>,
}

/// How long an optimistic drop stays overlaid on watch frames that do not
/// show it yet (the mutate round trip plus a publish); after that the board
/// trusts the engine again.
pub const OPTIMISTIC_TTL: Duration = Duration::from_secs(3);

/// A drop applied locally, waiting for the engine's frame to confirm it.
#[derive(Debug, Clone)]
struct PendingMove {
    plan: DropPlan,
    since: Instant,
}

#[derive(Debug, Default)]
pub struct BoardModel {
    tasks: Vec<Task>,
    selected: Option<String>,
    pending: Option<PendingMove>,
    /// Snapshots applied from the watch (proof counter).
    pub applied: usize,
    /// Frames on which a pending drop was re-overlaid (proof counter).
    pub overlaid: usize,
}

impl BoardModel {
    /// Replace the board with a watch snapshot. Keeps the selection when the
    /// task still exists. A pending optimistic drop stays overlaid until a
    /// frame shows it (or it ages out), so the card never jumps back and
    /// forth while the two mutates land.
    pub fn apply(&mut self, tasks: Vec<Task>) {
        self.apply_at(tasks, Instant::now());
    }

    pub fn apply_at(&mut self, mut tasks: Vec<Task>, now: Instant) {
        sort_tasks(&mut tasks);
        self.tasks = tasks;
        self.applied += 1;
        if let Some(id) = &self.selected
            && !self.tasks.iter().any(|t| &t.id == id)
        {
            self.selected = None;
        }
        if let Some(pending) = self.pending.clone() {
            let confirmed = self.plan_visible(&pending.plan);
            let expired = now.duration_since(pending.since) >= OPTIMISTIC_TTL;
            if confirmed || expired {
                self.pending = None;
            } else {
                self.apply_plan(&pending.plan);
                self.overlaid += 1;
            }
        }
    }

    /// Does the current board already show every write in `plan`?
    fn plan_visible(&self, plan: &DropPlan) -> bool {
        let Some(moved) = self.task(&plan.task_id) else {
            // Deleted meanwhile: nothing left to wait for.
            return true;
        };
        if let Some(status) = plan.status
            && moved.status != status
        {
            return false;
        }
        plan.ranks
            .iter()
            .all(|(id, rank)| self.task(id).is_none_or(|t| t.rank == *rank))
    }

    /// Apply a drop plan to the local rows (status + ranks), then re-sort.
    /// Ids are untouched, so a later frame replaces rows one for one.
    fn apply_plan(&mut self, plan: &DropPlan) {
        for task in &mut self.tasks {
            if task.id == plan.task_id
                && let Some(status) = plan.status
            {
                task.status = status;
            }
            if let Some((_, rank)) = plan.ranks.iter().find(|(id, _)| *id == task.id) {
                task.rank = *rank;
            }
        }
        sort_tasks(&mut self.tasks);
    }

    /// Show a drop immediately and remember it until the engine confirms.
    pub fn apply_optimistic(&mut self, plan: DropPlan, now: Instant) {
        self.apply_plan(&plan);
        self.pending = Some(PendingMove { plan, since: now });
    }

    /// Forget the pending drop (a mutate failed); the next frame is truth.
    pub fn clear_pending(&mut self) {
        self.pending = None;
    }

    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    pub fn tasks(&self) -> &[Task] {
        &self.tasks
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn task(&self, id: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    /// The cards of one column, board order.
    pub fn column(&self, status: TaskStatus) -> Vec<&Task> {
        self.tasks.iter().filter(|t| t.status == status).collect()
    }

    pub fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    pub fn select(&mut self, id: Option<String>) {
        self.selected = id;
    }

    /// Keyboard order: columns left to right, cards top to bottom.
    fn nav_order(&self) -> Vec<&str> {
        COLUMNS
            .iter()
            .flat_map(|s| self.column(*s))
            .map(|t| t.id.as_str())
            .collect()
    }

    /// Move the selection by `delta` cards; clamps at the ends. With nothing
    /// selected, down picks the first card and up the last.
    pub fn step(&mut self, delta: isize) {
        let order = self.nav_order();
        if order.is_empty() {
            self.selected = None;
            return;
        }
        let next = match self
            .selected
            .as_deref()
            .and_then(|id| order.iter().position(|o| *o == id))
        {
            Some(ix) => (ix as isize + delta).clamp(0, order.len() as isize - 1) as usize,
            None if delta < 0 => order.len() - 1,
            None => 0,
        };
        self.selected = Some(order[next].to_string());
    }

    /// Rank for a new card at the bottom of `status`.
    pub fn append_rank(&self, status: TaskStatus) -> f64 {
        self.column(status).last().map_or(1.0, |t| t.rank + 1.0)
    }

    /// Plan a drop. `None` when the drop changes nothing (dropped on itself
    /// or into the slot it already occupies).
    pub fn plan_drop(&self, task_id: &str, target: &DropTarget) -> Option<DropPlan> {
        let moved = self.task(task_id)?;
        let (status, before) = match target {
            DropTarget::Before(id) => {
                if id == task_id {
                    return None;
                }
                let anchor = self.task(id)?;
                (anchor.status, Some(anchor.id.as_str()))
            }
            DropTarget::End(status) => (*status, None),
        };
        // The column as it will look without the moved card.
        let column: Vec<&Task> = self
            .column(status)
            .into_iter()
            .filter(|t| t.id != task_id)
            .collect();
        let insert_at = match before {
            Some(id) => column.iter().position(|t| t.id == id)?,
            None => column.len(),
        };
        let same_column = moved.status == status;
        if same_column {
            let current = self
                .column(status)
                .iter()
                .position(|t| t.id == task_id)
                .unwrap_or(0);
            if current == insert_at {
                return None;
            }
        }
        let prev = insert_at.checked_sub(1).map(|i| column[i].rank);
        let next = column.get(insert_at).map(|t| t.rank);
        let mut plan = DropPlan {
            task_id: task_id.to_string(),
            status: (!same_column).then_some(status),
            ranks: Vec::new(),
        };
        match rank_between(prev, next) {
            Some(rank) => plan.ranks.push((task_id.to_string(), rank)),
            None => {
                // Stale ranks collided (equal neighbours, or no room left
                // between them): renumber the whole column 1, 2, 3 … with the
                // moved card in its slot.
                let mut ids: Vec<&str> = column.iter().map(|t| t.id.as_str()).collect();
                ids.insert(insert_at, task_id);
                plan.ranks
                    .push((task_id.to_string(), (insert_at + 1) as f64));
                for (ix, id) in ids.iter().enumerate() {
                    if *id != task_id {
                        plan.ranks.push((id.to_string(), (ix + 1) as f64));
                    }
                }
            }
        }
        Some(plan)
    }
}

/// The midpoint between two neighbour ranks, or the open end. `None` when the
/// neighbours leave no representable gap (equal, inverted, or too close).
pub fn rank_between(prev: Option<f64>, next: Option<f64>) -> Option<f64> {
    match (prev, next) {
        (None, None) => Some(1.0),
        (Some(p), None) => Some(p + 1.0),
        (None, Some(n)) => Some(n - 1.0),
        (Some(p), Some(n)) => {
            let mid = (p + n) / 2.0;
            (p < mid && mid < n).then_some(mid)
        }
    }
}

#[cfg(test)]
mod tests;
