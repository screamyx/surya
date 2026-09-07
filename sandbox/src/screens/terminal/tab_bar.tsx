// Rust: app/crates/ui/src/terminal/panel.rs (TerminalPanel::render_tab_bar at
// 1382, TabGhost at 325). An explicit split of the oversized panel.rs, the same
// move shell.tsx makes for shell.rs. Render helpers, not stateful components.
import { renderIcon } from '../../shell/icons';
import { renderTerminalIcon } from './icons';

/// One tab of the selected chat, mirroring TerminalTab's rendered fields.
/// `title` is already Self::display_title: the OSC title wins over "Terminal N".
export type TerminalTabModel = { key: string; title: string; exited: number | null };

/// The dragged-tab payload's paint (panel.rs TabGhost::render). Drag itself
/// does not port: gpui's on_drag has no admitted browser pair here.
export function TabGhost({ title }: { title: string }) {
  return (
    <div data-terminal="tab-ghost"
      className="absolute top-1.5 left-20 w-28 h-7 px-2 flex items-center rounded-md bg-surface-raised border border-border-strong text-ui-12 text-text">
      <div className="truncate">{title}</div>
    </div>
  );
}

/// One tab. Selected carries the ink(0.08) wash; a tab whose process exited
/// dims to 0.55; the close button only appears on the selected tab.
function renderTab(tab: TerminalTabModel, selected: boolean, dragged: boolean,
  onSelectTab: (key: string) => void, onCloseTab: (key: string) => void) {
  // The dragged tab leaves an invisible spacer: the ghost carries it.
  if (dragged) return <div key={tab.key} className="w-28 h-7 flex-none" />;
  return (
    <div key={tab.key} data-terminal-tab={tab.key} className={selected
      ? (tab.exited !== null
        ? 'w-28 h-7 flex-none flex flex-row items-center gap-1.5 pl-2 pr-1 rounded-lg cursor-pointer bg-text/10 text-ui-12 text-text opacity-55 hover:bg-element-hover active:bg-element-active'
        : 'w-28 h-7 flex-none flex flex-row items-center gap-1.5 pl-2 pr-1 rounded-lg cursor-pointer bg-text/10 text-ui-12 text-text hover:bg-element-hover active:bg-element-active')
      : (tab.exited !== null
        ? 'w-28 h-7 flex-none flex flex-row items-center gap-1.5 pl-2 pr-1 rounded-lg cursor-pointer text-ui-12 text-text-muted/50 opacity-55 hover:bg-element-hover active:bg-element-active'
        : 'w-28 h-7 flex-none flex flex-row items-center gap-1.5 pl-2 pr-1 rounded-lg cursor-pointer text-ui-12 text-text-muted/50 hover:bg-element-hover active:bg-element-active')}>
      <button type="button" aria-label={`Select terminal: ${tab.title}`}
        onClick={() => onSelectTab(tab.key)}
        className="flex-1 min-w-0 flex flex-row items-center gap-1.5 text-left cursor-pointer">
        <span className="size-4 flex-none">{renderTerminalIcon()}</span>
        <span className="flex-1 min-w-0 truncate">{tab.title}</span>
      </button>
      <button type="button" aria-label={`Close terminal: ${tab.title}`}
        onClick={() => onCloseTab(tab.key)}
        className={selected
          ? 'size-5 flex-none flex items-center justify-center rounded-md cursor-pointer text-text-muted/88 visible hover:bg-text/10 active:bg-element-active focus:bg-text/10'
          : 'size-5 flex-none flex items-center justify-center rounded-md cursor-pointer text-text-muted/88 invisible hover:bg-text/10 active:bg-element-active focus:bg-text/10'}>
        <span className="size-3 flex-none">{renderIcon('close')}</span>
      </button>
    </div>
  );
}

/// The tab strip: tabs, the new-tab button, then the collapse chevron pinned
/// right. The panel paints it only when it is not the embedded surface host.
export function renderTabBar(tabs: readonly TerminalTabModel[], activeKey: string, draggedKey: string | null,
  onSelectTab: (key: string) => void, onCloseTab: (key: string) => void,
  onNewTab: () => void, onToggleTerminal: () => void) {
  return (
    <div data-terminal="tab-bar"
      className="relative h-10 flex-none flex flex-row items-center gap-1 pl-2 pr-1.5 border-b border-border">
      {tabs.map(tab => renderTab(tab, tab.key === activeKey, tab.key === draggedKey, onSelectTab, onCloseTab))}
      <button type="button" data-terminal="new-tab" aria-label="New terminal"
        onClick={onNewTab}
        className="size-7 flex-none flex items-center justify-center rounded-lg cursor-pointer text-text-muted/50 hover:bg-text/5 active:bg-element-active focus:bg-text/5">
        <span className="size-4 flex-none">{renderIcon('plus')}</span>
      </button>
      <div className="flex-1" />
      <button type="button" data-terminal="collapse" aria-label="Hide terminal"
        onClick={onToggleTerminal}
        className="size-7 flex-none flex items-center justify-center rounded-lg cursor-pointer text-text-muted/50 hover:bg-text/5 active:bg-element-active focus:bg-text/5">
        <span className="size-3.5 flex-none">{renderIcon('alt-arrow-down')}</span>
      </button>
      {draggedKey !== null && <TabGhost title={tabs.find(tab => tab.key === draggedKey)?.title ?? ''} />}
    </div>
  );
}
