// Rust: app/crates/ui/src/tasks/edit.rs (EditSheet, TasksPane::render_sheet,
// parse_links, parse_owner) over the dialog primitives in popover.rs
// (modal, dialog_card, dialog_title, dialog_body, dialog_field, btn_ghost,
// btn_primary, btn_danger).
// The scrim is black at 0.6 dark / 0.32 light; no admitted token is black in
// both appearances. bg-on-solid/50 is black at 50 percent in dark, within a
// step of the native 0.6, and turns into a white veil in light.
// Notes and Links are multi-line ComposerInputs natively. They are single
// line here: a textarea draws a browser resize grabber and the whitelist has
// no resize utility to suppress it.
// Motion: popover.rs:663 modal_with wraps the card in motion::dialog_in, so the
// sheet fades up 2px over 180ms on mount. Natively that wrapper is a bare div
// around the frosted card; here the card carries the class itself, since the
// wrapper has nothing else on it. The three btn_ghost footers blend their
// label and fill over HOVER_FADE (popover.rs:998, :999, :1007); btn_primary
// and btn_danger use a plain .hover(opacity) and keep snapping.
import type { TaskStatus } from './model';
import { COLUMNS, columnLabel } from './model';

export type SheetState = {
  /// `null` = creating a new task.
  taskId: string | null;
  title: string;
  notes: string;
  owner: string;
  links: string;
  status: TaskStatus;
  confirmDelete: boolean;
};

export function newSheet(): SheetState {
  return { taskId: null, title: '', notes: '', owner: '', links: '', status: 'queued', confirmDelete: false };
}

/// Links are typed one per line or space-separated; keep the URLs.
export function parseLinks(text: string) {
  return text.split(/\s+/).filter(w => w.startsWith('http://') || w.startsWith('https://'));
}

/// Owner text to row value: empty / "you" / "user" = unowned (undefined).
export function parseOwner(text: string) {
  const owner = text.trim();
  if (owner === '' || owner.toLowerCase() === 'you' || owner.toLowerCase() === 'user') return undefined;
  return owner;
}

function field(label: string, placeholder: string, value: string, onValue: (next: string) => void, onSubmit: (() => void) | null) {
  return (
    <label className="mt-2.5 flex flex-col gap-1">
      <span className="text-ui-11 text-text-faint">{label}</span>
      <span className="w-full px-3 py-2 rounded-lg border border-wash/10 bg-wash/5 text-ui-14">
        <input type="text" value={value} placeholder={placeholder} className="w-full min-w-0 text-ui-14 text-text"
          onChange={event => onValue(event.target.value)}
          onKeyDown={event => { event.stopPropagation(); if (event.key === 'Enter' && onSubmit) { event.preventDefault(); onSubmit(); } }} />
      </span>
    </label>
  );
}

export function renderSheet(
  sheet: SheetState,
  onSheet: (next: SheetState) => void,
  onSubmit: () => void,
  onCancel: () => void,
  onDelete: () => void,
) {
  const editing = sheet.taskId !== null;
  return (
    <div className="absolute top-0 bottom-0 left-0 right-0 bg-on-solid/50 flex items-center justify-center">
      <div role="dialog" aria-label={editing ? 'Edit task' : 'New task'} data-sheet="task-edit-sheet"
        onKeyDown={event => { if (event.key === 'Escape') onCancel(); }}
        className="relative motion-dialog-in w-96 p-5 rounded-xl bg-surface-dialog border border-wash/10 flex flex-col text-text">
        <span className="text-ui-14 font-semibold text-text">{editing ? 'Edit task' : 'New task'}</span>
        {field('Title', 'Title', sheet.title, next => onSheet({ ...sheet, title: next }), onSubmit)}
        <div className="mt-2.5 flex flex-col gap-1.5">
          <span className="text-ui-11 text-text-faint">Status</span>
          <div className="flex flex-row flex-wrap gap-1.5">
            {COLUMNS.map(status => (
              <button key={status} type="button" data-sheet-status={status} aria-pressed={status === sheet.status}
                onClick={() => onSheet({ ...sheet, status })}
                className={status === sheet.status
                  ? 'px-2.5 py-1 rounded-full border border-accent bg-accent-wash text-ui-12 text-text cursor-pointer hover:bg-accent-wash focus:bg-accent-wash'
                  : 'px-2.5 py-1 rounded-full border border-wash/10 bg-wash/5 text-ui-12 text-text-muted cursor-pointer hover:bg-wash/10 focus:bg-wash/10'}>
                {columnLabel(status)}
              </button>
            ))}
          </div>
        </div>
        {field('Owner', 'Owner: agent id, or leave empty for you', sheet.owner, next => onSheet({ ...sheet, owner: next }), onSubmit)}
        {field('Notes', 'Notes (shift-enter for a new line)', sheet.notes, next => onSheet({ ...sheet, notes: next }), null)}
        {field('Links', 'Links: one URL per line', sheet.links, next => onSheet({ ...sheet, links: next }), null)}
        {sheet.confirmDelete ? (
          <div className="mt-4 flex flex-row items-center justify-between">
            <span className="text-ui-13 text-text-muted">Delete this task? Agents lose it too.</span>
            <div className="flex flex-row gap-2">
              <button type="button" onClick={() => onSheet({ ...sheet, confirmDelete: false })}
                className="px-3 py-1.5 rounded-lg text-ui-13 text-text-muted cursor-pointer motion-hover-fade hover:bg-wash/5 hover:text-text focus:bg-wash/5">Keep</button>
              <button type="button" onClick={onDelete}
                className="px-3 py-1.5 rounded-lg bg-danger-strong text-ui-13 font-medium text-on-accent cursor-pointer">Delete</button>
            </div>
          </div>
        ) : (
          <div className="mt-4 flex flex-row items-center justify-between">
            <div>
              {editing && <button type="button" onClick={() => onSheet({ ...sheet, confirmDelete: true })}
                className="px-3 py-1.5 rounded-lg text-ui-13 text-danger cursor-pointer motion-hover-fade hover:bg-wash/5 focus:bg-wash/5">Delete…</button>}
            </div>
            <div className="flex flex-row gap-2">
              <button type="button" onClick={onCancel}
                className="px-3 py-1.5 rounded-lg text-ui-13 text-text-muted cursor-pointer motion-hover-fade hover:bg-wash/5 hover:text-text focus:bg-wash/5">Cancel</button>
              <button type="button" onClick={onSubmit}
                className="px-3 py-1.5 rounded-lg bg-text text-ui-13 font-medium text-on-solid cursor-pointer">{editing ? 'Save' : 'Add task'}</button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
