// Rust: app/crates/ui/src/tasks/model.rs (COLUMNS, column_label, BoardModel
// column/nav_order/step) and the Task row of app/crates/proto/src/tasks.rs.
// Types and the pure grouping the render needs. Rank arithmetic, the drop
// plan and the optimistic overlay stay in Rust; see the report's Gaps.
export type TaskStatus = 'queued' | 'running' | 'done' | 'blocked';

export type BoardTask = {
  id: string;
  spaceId: string;
  title: string;
  status: TaskStatus;
  /// Agent id, or the literal `user`; absent = unowned.
  owner?: string;
  notes?: string;
  /// PR / issue / doc URLs.
  links: readonly string[];
  /// Board order within the space; ascending.
  rank: number;
  /// What card.rs formats out of `created_at`: local HH:MM.
  createdAt: string;
};

/// Column order on the board (brief: Queued, Running, Done, Blocked).
export const COLUMNS: readonly TaskStatus[] = ['queued', 'running', 'done', 'blocked'];

export function columnLabel(status: TaskStatus) {
  if (status === 'queued') return 'Queued';
  if (status === 'running') return 'Running';
  if (status === 'done') return 'Done';
  return 'Blocked';
}

/// The cards of one column, board order.
export function column(tasks: readonly BoardTask[], status: TaskStatus) {
  return tasks.filter(t => t.status === status);
}

/// Keyboard order: columns left to right, cards top to bottom.
export function navOrder(tasks: readonly BoardTask[]) {
  return COLUMNS.flatMap(s => column(tasks, s)).map(t => t.id);
}

/// Move the selection by `delta` cards; clamps at the ends. With nothing
/// selected, down picks the first card and up the last.
export function step(tasks: readonly BoardTask[], selected: string | null, delta: number) {
  const order = navOrder(tasks);
  if (order.length === 0) return null;
  const at = selected === null ? -1 : order.indexOf(selected);
  if (at < 0) return delta < 0 ? order[order.length - 1] : order[0];
  return order[Math.min(Math.max(at + delta, 0), order.length - 1)];
}
