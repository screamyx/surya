// Rust: app/crates/ui/src/tasks/board.rs (TasksPane, its Render impl and the
// render_column helper on the same struct).
// The watch, the RPC mutates and the optimistic drop overlay stay in Rust:
// props come in, intent callbacks go out. The root carries `relative` that
// the native root does not, because the sheet is a deferred anchored layer
// natively and an absolute child here.
import { useState } from 'react';
import type { BoardTask, TaskStatus } from './tasks/model';
import { COLUMNS, column, columnLabel, step } from './tasks/model';
import type { SheetState } from './tasks/edit';
import { countChip } from './tasks/chips';
import { renderCard, renderCardGhost } from './tasks/card';
import { renderHeader } from './tasks/header';
import { renderQuickAdd } from './tasks/quick_add';
import { newSheet, parseLinks, parseOwner, renderSheet } from './tasks/edit';
import { COLUMN_GAP, COLUMN_MIN_W, entries, renderScroller, renderStrip } from './tasks/columns_strip';

export type TaskDraft = {
  title: string;
  status: TaskStatus;
  notes?: string;
  owner?: string;
  links: readonly string[];
};

export type TasksProps = {
  spaceName: string;
  tasks: readonly BoardTask[];
  /// Last RPC failure, shown under the header until the next success.
  error?: string;
  /// Measured width of the columns row. 0 is the unmeasured first frame, and
  /// the strip then reports every column visible, exactly as the native one
  /// does rather than dimming all four.
  boardViewport?: number;
  /// How far the columns row is scrolled, for the same reason.
  scrolled?: number;
  /// CardGhost, the floating copy under the pointer. Drag is a native
  /// lifecycle with no admitted pair, so a fixture prop shows the paint.
  ghostTitle?: string;
  onCreateTask: (draft: TaskDraft) => void;
  onUpdateTask: (taskId: string, draft: TaskDraft) => void;
  onDeleteTask: (taskId: string) => void;
};

export function TasksPane({
  spaceName, tasks, error, boardViewport, scrolled, ghostTitle,
  onCreateTask, onUpdateTask, onDeleteTask,
}: TasksProps) {
  const [selected, setSelected] = useState<string | null>(null);
  const [quickAdd, setQuickAdd] = useState('');
  const [sheet, setSheet] = useState<SheetState | null>(null);

  function openSheet(taskId: string | null) {
    const task = taskId === null ? undefined : tasks.find(t => t.id === taskId);
    if (task === undefined) { setSheet(newSheet()); return; }
    setSheet({
      taskId: task.id, title: task.title, notes: task.notes ?? '', owner: task.owner ?? '',
      links: task.links.join('\n'), status: task.status, confirmDelete: false,
    });
  }

  /// Enter submits, a blank title is not a task, so the sheet stays open.
  function submitSheet() {
    if (sheet === null) return;
    const title = sheet.title.trim();
    if (title === '') return;
    const notes = sheet.notes.trim();
    const draft: TaskDraft = {
      title, status: sheet.status, links: parseLinks(sheet.links),
      notes: notes === '' ? undefined : notes, owner: parseOwner(sheet.owner),
    };
    if (sheet.taskId === null) onCreateTask(draft); else onUpdateTask(sheet.taskId, draft);
    setSheet(null);
  }

  function deleteFromSheet() {
    if (sheet === null) return;
    if (sheet.taskId !== null) onDeleteTask(sheet.taskId);
    setSheet(null);
  }

  /// A blank title is a no-op, so enter on an empty box costs nothing.
  function submitQuickAdd() {
    const title = quickAdd.trim();
    if (title === '') return;
    setQuickAdd('');
    onCreateTask({ title, status: 'queued', links: [] });
  }

  function onKey(key: string) {
    if (key === 'ArrowUp') setSelected(step(tasks, selected, -1));
    else if (key === 'ArrowDown') setSelected(step(tasks, selected, 1));
    else if (key === 'Enter') { if (selected !== null) openSheet(selected); }
    else if (key === 'n') openSheet(null);
    else if (key === 'Escape') setSheet(null);
  }

  function renderColumn(status: TaskStatus) {
    const cards = column(tasks, status);
    return (
      <div key={status} data-column={status} className="flex-1 min-w-55 flex flex-col gap-2">
        <div className="flex flex-row items-center gap-1.5 px-0.5">
          <span className="text-ui-13 font-semibold text-text">{columnLabel(status)}</span>
          {countChip(cards.length)}
        </div>
        <div className="flex-1 min-h-40 p-2 rounded-lg bg-surface flex flex-col gap-2 overflow-y-scroll">
          {status === 'queued' && renderQuickAdd(quickAdd, setQuickAdd, submitQuickAdd)}
          {cards.map(task => renderCard(task, selected === task.id, setSelected, openSheet))}
        </div>
      </div>
    );
  }

  const strip = entries(
    status => column(tasks, status).length,
    scrolled ?? 0, boardViewport ?? 0, COLUMN_MIN_W, COLUMN_GAP,
  );
  return (
    <section aria-labelledby="tasks-heading" data-pane="tasks-pane" tabIndex={0}
      onKeyDown={event => {
        if (['ArrowUp', 'ArrowDown', 'Enter', 'n', 'Escape'].includes(event.key)) event.preventDefault();
        onKey(event.key);
      }}
      className="size-full relative flex flex-col bg-bg text-text">
      {renderHeader(spaceName, tasks.length)}
      {renderStrip(strip)}
      {error !== undefined && error !== '' && (
        <div role="status" className="mx-4 mb-2 px-2.5 py-1.5 rounded-lg bg-danger-muted text-ui-12 text-danger">{error}</div>
      )}
      {renderScroller(COLUMNS.map(status => renderColumn(status)))}
      {ghostTitle !== undefined && renderCardGhost(ghostTitle)}
      {sheet !== null && renderSheet(sheet, setSheet, submitSheet, () => setSheet(null), deleteFromSheet)}
    </section>
  );
}
