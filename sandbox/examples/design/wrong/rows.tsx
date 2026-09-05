// Rust: inbox/needs_you/rows.rs, NeedsYouPane::render_collapsed_row.
// Render helper on the same pane, not a second stateful component.
import type { InboxRow } from '../../../src/screens/NeedsYou';
import { badge, hintChip } from '../../../src/screens/chrome';
export function renderRow(row: InboxRow, first: boolean, onOpenChat: (id: string) => void) {
  return (
    <button key={row.id} type="button" onClick={() => onOpenChat(row.chatId)}
      aria-label={`Open chat: ${row.prompt}`} data-row={row.id} style={{ paddingLeft: 4, paddingRight: 4 }}
      className={first
        ? 'flex flex-row items-center gap-2 py-2.5 text-left cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover'
        : 'flex flex-row items-center gap-2 py-2.5 text-left border-t border-border cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover'}>
      {badge('Question', 'warning')}
      <span className="flex-1 min-w-0 truncate whitespace-nowrap text-ui-13 text-text">{row.prompt}</span>
      {hintChip('answer below')}
    </button>
  );
}
