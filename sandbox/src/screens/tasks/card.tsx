// Rust: app/crates/ui/src/tasks/card.rs (TasksPane::render_card, CardGhost).
// The drag source and drop target are a native gpui lifecycle with no
// admitted pair, so the card here selects, opens and hovers but never drags,
// and the ghost is a static state a fixture prop turns on. shadow_sm and
// shadow_lg have no pair; cursor_grab falls back to cursor-pointer.
import type { BoardTask } from './model';
import { linkChip, ownerChip, preview, shortId } from './chips';

export function renderCard(
  task: BoardTask,
  selected: boolean,
  onSelect: (taskId: string) => void,
  onOpen: (taskId: string) => void,
) {
  const added = `${shortId(task.id)} · added ${task.createdAt}`;
  const notes = (task.notes ?? '').trim();
  return (
    <button key={task.id} type="button" data-card={task.id} aria-pressed={selected}
      onClick={() => onSelect(task.id)} onDoubleClick={() => onOpen(task.id)}
      className={selected
        ? 'flex flex-col gap-2 px-3 py-2.5 rounded-lg text-left bg-surface-card border border-accent cursor-pointer hover:bg-surface-raised-hover'
        : 'flex flex-col gap-2 px-3 py-2.5 rounded-lg text-left bg-surface-card border border-wash/10 cursor-pointer hover:bg-surface-raised-hover'}>
      <span className="text-ui-13 font-medium text-text">{task.title}</span>
      {task.status === 'running' && <span className="h-0.75 w-28 flex-none rounded-sm bg-accent" />}
      {notes !== '' && <span className="text-ui-12 text-text-muted">{preview(notes, 90)}</span>}
      <span className="flex flex-row flex-wrap gap-1.5">
        {ownerChip(task.owner)}
        {task.links.map(link => <span key={link}>{linkChip(link)}</span>)}
      </span>
      <span className="text-ui-11 text-text-faint">{added}</span>
    </button>
  );
}

/// The floating copy under the pointer while dragging. Static here: React
/// drag events are not an admitted pair, so a fixture prop shows the paint.
export function renderCardGhost(title: string) {
  return (
    <div data-card-ghost="1" aria-hidden="true"
      className="absolute top-96 left-40 w-64 px-3 py-2.5 rounded-lg bg-surface-card border border-border-strong text-ui-13 text-text">
      {title}
    </div>
  );
}
