//! Board model behind the Tasks pane: pure data, no gpui. Groups the
//! `WatchTasks` snapshot into columns, tracks the keyboard selection, and
//! does the rank arithmetic for a drop (one row moves; a collision between
//! stale ranks renumbers the column).

use zeron_proto::{Task, TaskStatus, sort_tasks};

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

#[derive(Debug, Default)]
pub struct BoardModel {
    tasks: Vec<Task>,
    selected: Option<String>,
    /// Snapshots applied from the watch (proof counter).
    pub applied: usize,
}

impl BoardModel {
    /// Replace the board with a watch snapshot. Keeps the selection when the
    /// task still exists, else moves it to the first card.
    pub fn apply(&mut self, mut tasks: Vec<Task>) {
        sort_tasks(&mut tasks);
        self.tasks = tasks;
        self.applied += 1;
        if let Some(id) = &self.selected
            && !self.tasks.iter().any(|t| &t.id == id)
        {
            self.selected = None;
        }
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
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};

    fn at(ms: i64) -> DateTime<Utc> {
        DateTime::from_timestamp_millis(ms).unwrap()
    }

    fn task(id: &str, status: TaskStatus, rank: f64) -> Task {
        Task {
            id: id.into(),
            space_id: "sp".into(),
            title: format!("Task {id}"),
            status,
            owner: None,
            notes: None,
            links: vec![],
            rank,
            created_at: at(1),
            updated_at: at(1),
        }
    }

    fn board() -> BoardModel {
        let mut m = BoardModel::default();
        m.apply(vec![
            task("q2", TaskStatus::Queued, 2.0),
            task("q1", TaskStatus::Queued, 1.0),
            task("r1", TaskStatus::Running, 1.0),
            task("d1", TaskStatus::Done, 5.0),
            task("b1", TaskStatus::Blocked, 1.0),
            task("q3", TaskStatus::Queued, 3.0),
        ]);
        m
    }

    #[test]
    fn groups_into_columns_in_rank_order() {
        let m = board();
        let ids = |s| m.column(s).iter().map(|t| t.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(TaskStatus::Queued), ["q1", "q2", "q3"]);
        assert_eq!(ids(TaskStatus::Running), ["r1"]);
        assert_eq!(ids(TaskStatus::Done), ["d1"]);
        assert_eq!(ids(TaskStatus::Blocked), ["b1"]);
        assert_eq!(m.append_rank(TaskStatus::Queued), 4.0);
        assert_eq!(m.append_rank(TaskStatus::Done), 6.0);
    }

    #[test]
    fn watch_apply_counts_every_snapshot_and_keeps_a_live_selection() {
        let mut m = BoardModel::default();
        m.select(Some("q1".into()));
        let events = 6;
        for i in 0..events {
            let mut tasks = vec![task("q1", TaskStatus::Queued, 1.0)];
            if i % 2 == 0 {
                tasks.push(task("x", TaskStatus::Running, 1.0));
            }
            m.apply(tasks);
            assert_eq!(m.selected(), Some("q1"));
        }
        println!("events={events} applied={}", m.applied);
        assert_eq!(m.applied, events);
        m.apply(vec![task("z", TaskStatus::Done, 1.0)]);
        assert_eq!(m.selected(), None, "selection dropped with its task");
    }

    #[test]
    fn keyboard_steps_across_columns_and_clamps() {
        let mut m = board();
        m.step(1);
        assert_eq!(m.selected(), Some("q1"));
        m.step(1);
        m.step(1);
        m.step(1);
        assert_eq!(m.selected(), Some("r1"), "q3 -> r1 crosses the column");
        m.step(10);
        assert_eq!(m.selected(), Some("b1"), "clamps at the last card");
        m.step(-100);
        assert_eq!(m.selected(), Some("q1"));
        m.select(None);
        m.step(-1);
        assert_eq!(m.selected(), Some("b1"), "up from nothing picks the last");
    }

    #[test]
    fn drop_before_a_card_takes_the_midpoint() {
        let m = board();
        let plan = m.plan_drop("q3", &DropTarget::Before("q2".into())).unwrap();
        assert_eq!(plan.status, None);
        assert_eq!(plan.ranks, vec![("q3".to_string(), 1.5)]);
        // Dropping at the very top goes below the first rank.
        let plan = m.plan_drop("q3", &DropTarget::Before("q1".into())).unwrap();
        assert_eq!(plan.ranks, vec![("q3".to_string(), 0.0)]);
    }

    #[test]
    fn drop_into_another_column_changes_status_and_appends() {
        let m = board();
        let plan = m
            .plan_drop("q1", &DropTarget::End(TaskStatus::Running))
            .unwrap();
        assert_eq!(plan.status, Some(TaskStatus::Running));
        assert_eq!(plan.ranks, vec![("q1".to_string(), 2.0)]);
        let plan = m.plan_drop("q1", &DropTarget::Before("r1".into())).unwrap();
        assert_eq!(plan.status, Some(TaskStatus::Running));
        assert_eq!(plan.ranks, vec![("q1".to_string(), 0.0)]);
    }

    #[test]
    fn no_op_drops_plan_nothing() {
        let m = board();
        assert_eq!(m.plan_drop("q2", &DropTarget::Before("q2".into())), None);
        assert_eq!(
            m.plan_drop("q2", &DropTarget::Before("q3".into())),
            None,
            "already there"
        );
        assert_eq!(
            m.plan_drop("q3", &DropTarget::End(TaskStatus::Queued)),
            None,
            "already last"
        );
        assert_eq!(
            m.plan_drop("nope", &DropTarget::End(TaskStatus::Queued)),
            None
        );
    }

    #[test]
    fn stale_rank_collision_renumbers_the_column() {
        let mut m = BoardModel::default();
        // Two devices appended "at the bottom" at the same time: equal ranks.
        m.apply(vec![
            task("a", TaskStatus::Queued, 1.0),
            task("b", TaskStatus::Queued, 1.0),
            task("c", TaskStatus::Queued, 1.0),
        ]);
        let plan = m.plan_drop("c", &DropTarget::Before("b".into())).unwrap();
        assert_eq!(
            plan.ranks,
            vec![
                ("c".to_string(), 2.0),
                ("a".to_string(), 1.0),
                ("b".to_string(), 3.0),
            ],
            "moved card first, then the mates, ranks 1..=3"
        );
        assert_eq!(rank_between(Some(1.0), Some(1.0)), None);
        assert_eq!(
            rank_between(Some(2.0), Some(1.0)),
            None,
            "inverted neighbours"
        );
        assert_eq!(rank_between(Some(1.0), Some(2.0)), Some(1.5));
        assert_eq!(rank_between(None, None), Some(1.0));
    }
}
