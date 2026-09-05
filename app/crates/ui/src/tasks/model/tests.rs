//! Board model tests: grouping, keyboard, drop rank math, optimistic apply.

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
fn optimistic_drop_shows_at_once_and_survives_partial_frames() {
    let mut m = board();
    let t0 = Instant::now();
    let plan = m
        .plan_drop("q1", &DropTarget::End(TaskStatus::Running))
        .unwrap();
    m.apply_optimistic(plan.clone(), t0);
    let running = |m: &BoardModel| {
        m.column(TaskStatus::Running)
            .iter()
            .map(|t| t.id.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(running(&m), ["r1", "q1"], "moved before any frame");
    assert_eq!(m.len(), 6, "no duplicate card");

    // Frame 1: the engine has not published yet (old state).
    let stale: Vec<Task> = board().tasks().to_vec();
    m.apply_at(stale.clone(), t0 + Duration::from_millis(50));
    assert_eq!(running(&m), ["r1", "q1"], "overlay keeps the move");
    assert_eq!(m.len(), 6);
    assert!(m.has_pending());

    // Frame 2: status landed, rank not yet (the second mutate in flight).
    let mut partial = stale.clone();
    partial.iter_mut().find(|t| t.id == "q1").unwrap().status = TaskStatus::Running;
    m.apply_at(partial, t0 + Duration::from_millis(120));
    assert_eq!(
        running(&m),
        ["r1", "q1"],
        "rank overlay still applied (q1 rank 1.0 would sort first)"
    );
    assert!(m.has_pending());

    // Frame 3: both writes visible: pending clears, frame is truth.
    let mut done = stale.clone();
    {
        let q1 = done.iter_mut().find(|t| t.id == "q1").unwrap();
        q1.status = TaskStatus::Running;
        q1.rank = 2.0;
    }
    m.apply_at(done, t0 + Duration::from_millis(200));
    assert!(!m.has_pending(), "confirmed by the frame");
    assert_eq!(running(&m), ["r1", "q1"]);
    println!("frames=3 overlaid={} confirmed=1", m.overlaid);
    assert_eq!(m.overlaid, 2);
}

#[test]
fn optimistic_drop_expires_and_clears_on_failure() {
    let mut m = board();
    let t0 = Instant::now();
    let plan = m.plan_drop("q3", &DropTarget::Before("q1".into())).unwrap();
    m.apply_optimistic(plan, t0);
    let stale: Vec<Task> = board().tasks().to_vec();
    m.apply_at(stale.clone(), t0 + OPTIMISTIC_TTL);
    assert!(!m.has_pending(), "aged out: the engine wins");
    let queued: Vec<String> = m
        .column(TaskStatus::Queued)
        .iter()
        .map(|t| t.id.clone())
        .collect();
    assert_eq!(queued, ["q1", "q2", "q3"], "back to the engine's order");

    let plan = m.plan_drop("q3", &DropTarget::Before("q1".into())).unwrap();
    m.apply_optimistic(plan, t0);
    m.clear_pending();
    m.apply_at(stale, t0);
    assert_eq!(m.overlaid, 0);
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
