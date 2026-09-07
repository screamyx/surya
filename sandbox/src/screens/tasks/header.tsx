// Rust: app/crates/ui/src/tasks/header.rs (render, count_label, HINT).
// The gap between the count and the hint is HEADER_GAP = Theme::SPACE_LG = 16,
// the only thing holding the two strings apart once justify_between gives up.
// The title is ui_rems(30.0) natively; text-ui-20 is the largest admitted step.

/// The line under the count. Never elided: it is short, and half of it is
/// no use.
const HINT = 'Agents pull from Queued when they go idle.';

/// "1 task", "2 tasks", "0 tasks".
export function countLabel(count: number) {
  return `${count} task${count === 1 ? '' : 's'}`;
}

/// The header row: `<project name> <n tasks>` on the left, the hint right.
export function renderHeader(spaceName: string, count: number) {
  return (
    <div className="flex flex-row items-end justify-between gap-4 px-4 pt-6 pb-3">
      <div className="flex flex-row items-end gap-2 min-w-0">
        <h1 id="tasks-heading" className="min-w-0 truncate text-ui-20 font-semibold text-text">{spaceName}</h1>
        <span className="flex-none pb-1.5 text-ui-13 text-text-muted">{countLabel(count)}</span>
      </div>
      <span className="flex-none pb-1.5 text-ui-13 text-text-muted">{HINT}</span>
    </div>
  );
}
