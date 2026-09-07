// Rust: app/crates/ui/src/inbox/agents.rs (AgentsRail, its Render impl).
// Row types mirror app/crates/ui/src/inbox/model.rs (AgentRow, AgentSection,
// AgentGroup); the grouping and roll-up logic stays in Rust and arrives as props.
import { useState } from 'react';
import { renderRow } from './agents/rows';
import { emptyState, groupHeading } from './agents/chrome';

/// Rust: surya_proto::AgentState, flattened with its NeedsYou kind.
export type AgentStateKind =
  | 'needs-you-permission'
  | 'needs-you-question'
  | 'needs-you-failed'
  | 'stopped'
  | 'working'
  | 'done'
  | 'idle';

/// Rust: model.rs AgentGroup, in decision 15's order.
export type AgentGroup = 'waiting-for-you' | 'running' | 'done' | 'idle';

/// One drawn row of the agent tree: the state row plus how deep it sits.
/// Rust: model.rs AgentRow.
export type AgentRow = {
  id: string;
  chatId: string;
  /// Chat title for a top-level agent, the spawn's description for a child.
  name: string;
  /// Nesting depth. 0 is a top-level agent.
  depth: number;
  state: AgentStateKind;
  rolledUp: AgentStateKind;
  needsYouChildren: number;
  /// True when the row's own state is calm but a descendant is not.
  rolledUpOnly: boolean;
};

/// One group heading plus its rows, children already nested under parents.
/// Rust: model.rs AgentSection.
export type AgentSection = { group: AgentGroup; rows: readonly AgentRow[] };

export type AgentsRailProps = {
  sections: readonly AgentSection[];
  /// The chat the shell is showing. Rust: AgentsRail::set_selected.
  selectedChatId?: string;
  /// Rust: cx.emit(SelectChat(chat_id)). The shell owns routing, not this pane.
  onSelectChat: (chatId: string) => void;
};

export function AgentsRail({ sections, selectedChatId, onSelectChat }: AgentsRailProps) {
  // Rust keeps the highlight in AgentsRail::selected and lets the shell set it.
  // Local state here so a click lights the row in the sandbox as it does natively.
  const [selected, setSelected] = useState<string | undefined>(selectedChatId);
  function select(chatId: string) {
    setSelected(chatId);
    onSelectChat(chatId);
  }
  return (
    <nav aria-label="Agents" data-pane="agents-rail" className="size-full overflow-y-scroll flex flex-col p-2">
      {sections.length === 0 ? emptyState('No agents yet.') : sections.map(section => (
        <div key={section.group} className="flex flex-col">
          {groupHeading(section.group)}
          {section.rows.map(row => renderRow(row, selected === row.chatId, select))}
        </div>
      ))}
    </nav>
  );
}
