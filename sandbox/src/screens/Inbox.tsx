// Rust: app/crates/ui/src/inbox/mod.rs (InboxPane, its Render impl).
// The inbox surface: the needs-you list with the agent tree beside it.
import type { AgentSection } from './Agents';
import type { InboxRow } from './NeedsYou';
import { AgentsRail } from './Agents';
import { NeedsYouPane } from './NeedsYou';

export type InboxPaneProps = {
  sections: readonly AgentSection[];
  rows: readonly InboxRow[];
  selectedChatId?: string;
  onSelectChat: (chatId: string) => void;
  onOpenChat: (chatId: string) => void;
};

export function InboxPane({ sections, rows, selectedChatId, onSelectChat, onOpenChat }: InboxPaneProps) {
  return (
    <div className="size-full flex flex-row bg-bg">
      <div className="w-64 h-full flex-none border-r border-border">
        <AgentsRail sections={sections} selectedChatId={selectedChatId} onSelectChat={onSelectChat} />
      </div>
      <div className="flex-1 min-w-0 h-full">
        <NeedsYouPane rows={rows} onOpenChat={onOpenChat} />
      </div>
    </div>
  );
}
