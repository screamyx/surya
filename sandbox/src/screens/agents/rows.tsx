// Rust: app/crates/ui/src/inbox/agents.rs, AgentsRail::render_row.
// Render helper on the same pane, not a second stateful component.
import type { AgentRow } from '../Agents';
import { badge, dot, stateTone } from './chrome';

// Children step in; the indent IS the tree. Rust: .pl(px(6.0 + depth * 14.0)).
// Four admitted steps: 6px, 20px, 32px (native 34px) and 48px. Deeper rows
// clamp to the last step rather than inventing a width the whitelist lacks.
export function renderRow(row: AgentRow, selected: boolean, onSelectChat: (chatId: string) => void) {
  return (
    <button key={row.id} type="button" data-row={row.id} data-depth={row.depth}
      aria-current={selected ? 'true' : undefined}
      onClick={() => onSelectChat(row.chatId)}
      className={selected
        ? (row.depth <= 0
          ? 'flex flex-row items-center gap-2 pl-1.5 pr-1.5 py-1.5 rounded-lg text-left cursor-pointer bg-element-active'
          : row.depth === 1
            ? 'flex flex-row items-center gap-2 pl-5 pr-1.5 py-1.5 rounded-lg text-left cursor-pointer bg-element-active'
            : row.depth === 2
              ? 'flex flex-row items-center gap-2 pl-8 pr-1.5 py-1.5 rounded-lg text-left cursor-pointer bg-element-active'
              : 'flex flex-row items-center gap-2 pl-12 pr-1.5 py-1.5 rounded-lg text-left cursor-pointer bg-element-active')
        : (row.depth <= 0
          ? 'flex flex-row items-center gap-2 pl-1.5 pr-1.5 py-1.5 rounded-lg text-left cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover'
          : row.depth === 1
            ? 'flex flex-row items-center gap-2 pl-5 pr-1.5 py-1.5 rounded-lg text-left cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover'
            : row.depth === 2
              ? 'flex flex-row items-center gap-2 pl-8 pr-1.5 py-1.5 rounded-lg text-left cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover'
              : 'flex flex-row items-center gap-2 pl-12 pr-1.5 py-1.5 rounded-lg text-left cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover')}>
      {dot(stateTone(row.state))}
      <span className={selected
        ? 'flex-1 min-w-0 truncate whitespace-nowrap text-ui-13 text-text'
        : 'flex-1 min-w-0 truncate whitespace-nowrap text-ui-13 text-text-muted'}>{row.name}</span>
      {/* The roll-up badge: this row is calm, something under it is not. */}
      {row.rolledUpOnly && badge(String(row.needsYouChildren), stateTone(row.rolledUp))}
    </button>
  );
}
