// Rust: app/crates/ui/src/terminal/panel.rs (impl Render for TerminalPanel at
// 1640). Tabs are per selected chat; the emulator, the PTY stream and every RPC
// stay in Rust. This paints one tab's snapshot.
import { useState } from 'react';
import { renderTabBar } from './terminal/tab_bar';
import type { TerminalTabModel } from './terminal/tab_bar';
import { renderScrollbar } from './terminal/scrollbar';
import { renderTerminalView } from './terminal/view';
import type { TermCursor, TermRow } from './terminal/view';

/// One tab plus the grid snapshot its emulator would hand the paint element.
/// `historyLines` is what scrollbar_metrics needs to decide the rail exists.
export type TerminalTab = TerminalTabModel & {
  rows: readonly TermRow[];
  cursor: TermCursor | null;
  historyLines: number;
};

export type TerminalPanelProps = {
  tabs: readonly TerminalTab[];
  /// Initial TerminalPanel.chats[chat].active. Tab selection is panel-local in
  /// Rust too, so it becomes local state here rather than a controlled prop.
  activeKey: string;
  /// False when no chat is selected: the panel paints its one-line placeholder.
  selectedChat: boolean;
  /// Right-pane surface host: the shell owns the tab strip and the pane fill.
  embedded?: boolean;
  /// Stands in for a pointer drag over the strip. Drag itself does not port.
  draggedKey?: string | null;
  /// panel.rs's `over`: the slot the dragged tab is currently above. With
  /// `draggedKey` it is the whole of the native drag state, and the two
  /// together drive the TAB_SLIDE tween on the siblings.
  dragOverKey?: string | null;
  onNewTab: () => void;
  onCloseTab: (key: string) => void;
  onToggleTerminal: () => void;
};

export function TerminalPanel({ tabs, activeKey, selectedChat, embedded = false, draggedKey = null,
  dragOverKey = null, onNewTab, onCloseTab, onToggleTerminal }: TerminalPanelProps) {
  const [selected, setSelected] = useState(activeKey);
  const [terminalHovered, setTerminalHovered] = useState(false);
  const [scrollbarHovered, setScrollbarHovered] = useState(false);
  const [focused, setFocused] = useState(false);
  const active = tabs.find(tab => tab.key === selected) ?? tabs[0];

  if (!selectedChat) {
    return (
      <div data-terminal="panel" className={embedded
        ? 'size-full flex items-center justify-center text-ui-12 text-text-faint'
        : 'size-full flex items-center justify-center bg-terminal-background text-ui-12 text-text-faint'}>
        Select a chat to open a terminal
      </div>
    );
  }
  return (
    <div data-terminal="panel" className={embedded
      ? 'size-full flex flex-col'
      : 'size-full flex flex-col bg-terminal-background'}>
      {!embedded && renderTabBar(tabs, active?.key ?? '', draggedKey, dragOverKey,
        key => { setSelected(key); setFocused(false); },
        key => { onCloseTab(key); setFocused(false); },
        () => { onNewTab(); setFocused(false); },
        () => { onToggleTerminal(); setFocused(false); })}
      {/* Native focuses the panel on the press that lands in the body, and the
          chrome above takes focus back; the cursor fills only while focused. */}
      <div data-terminal="body"
        onMouseEnter={() => setTerminalHovered(true)}
        onMouseLeave={() => { setTerminalHovered(false); setScrollbarHovered(false); }}
        onMouseDown={() => setFocused(true)}
        className="relative flex-1 min-h-0 cursor-default">
        {renderTerminalView(active?.rows ?? [], active?.cursor ?? null, focused)}
        {terminalHovered && active !== undefined && active.historyLines > 0
          && renderScrollbar(scrollbarHovered, setScrollbarHovered)}
      </div>
    </div>
  );
}
