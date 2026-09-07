// Rust: app/crates/ui/src/shell/tabs.rs render_session_title_bar (130) and
// right_pane_expand_icon (35), app/crates/ui/src/shell.rs
// render_right_tab_strip (7591), SurfaceTabGhost (826) and DragGhost (851).
//
// The horizontal SESSION tab strip is gone (tabs.rs header, wing 2026-08-10):
// the sidebar IS the session list and the titlebar names the selected one.
// What remains under "tab strip" is the right pane's surface chips.
import type { SurfaceTab } from '../../fixtures/spaces';
import { menuRow, popoverCard } from '../popover';
import { renderIcon } from './icons';

/// cycle_target: the chat one step from the selection in the sidebar order,
/// wrapping at both ends. Nothing selected enters the list at the end it
/// would have wrapped to. Pure, so it ports whole.
export function cycleTarget(order: readonly string[], selected: string | null, forward: boolean) {
  if (order.length === 0) return null;
  const at = selected === null ? -1 : order.indexOf(selected);
  if (at < 0) return forward ? order[0] : order[order.length - 1];
  const next = forward ? (at + 1) % order.length : (at + order.length - 1) % order.length;
  return order[next];
}

export function rightPaneExpandIcon(expanded: boolean) {
  return expanded ? 'collapse-arrows' : 'expand-arrows';
}

/// SurfaceTabGhost: the chip that follows the pointer while a surface tab
/// drags. Drag does not port, so this paints as a toggleable state.
export function surfaceTabGhost(title: string) {
  return <div data-ghost="surface-tab" className="h-6 w-28 px-2 flex items-center rounded-md bg-surface-raised border border-border-strong text-ui-11 text-text opacity-100">
    <span className="min-w-0 truncate whitespace-nowrap">{title}</span>
  </div>;
}

/// DragGhost: resize drags render nothing at the cursor. Kept as a named
/// helper so port-back can find its pair.
export function dragGhost() {
  return null;
}

export type TabStripProps = {
  tabs: readonly SurfaceTab[];
  active: string;
  hovered: string | null;
  plusOpen: boolean;
  ghost: string | null;
  gitDetected: boolean;
  onSelect: (key: string) => void;
  onClose: (key: string) => void;
  onHover: (key: string | null) => void;
  onTogglePlus: () => void;
  onAddSurface: (kind: 'terminal' | 'diff') => void;
};

/// render_right_tab_strip: fixed 112px chips in a scrolling row, each with a
/// leading slot whose surface mark swaps IN PLACE for the close mark on chip
/// hover, and a trailing `+` offering the two surfaces.
export function renderRightTabStrip(p: TabStripProps) {
  return (
    <div className="flex-1 min-w-0 flex flex-row items-center gap-1 overflow-x-scroll" data-strip="right-surface">
      {p.tabs.map(tab => (
        <div key={tab.key} data-tab={tab.key}
          onMouseEnter={() => p.onHover(tab.key)} onMouseLeave={() => p.onHover(null)}
          className={tab.key === p.active
            ? 'h-6 w-28 flex-none pl-1 pr-2 rounded-md flex flex-row items-center gap-0.75 cursor-pointer bg-wash/10'
            : 'h-6 w-28 flex-none pl-1 pr-2 rounded-md flex flex-row items-center gap-0.75 cursor-pointer hover:bg-wash/5'}>
          <button type="button" data-tab-close={tab.key}
            aria-label={'Close ' + tab.title}
            onClick={() => (p.hovered === tab.key ? p.onClose(tab.key) : p.onSelect(tab.key))}
            className="flex-none size-5 rounded-sm flex items-center justify-center cursor-pointer hover:bg-wash/14 active:bg-wash/14 focus:bg-wash/14">
            {p.hovered === tab.key
              ? <span className="size-3 text-text-muted">{renderIcon('close')}</span>
              : <span className={tab.key === p.active ? 'size-3 text-text-muted' : 'size-3 text-text-muted/88'}>{renderIcon(tab.icon)}</span>}
          </button>
          <button type="button" onClick={() => p.onSelect(tab.key)} data-tab-select={tab.key}
            className={tab.key === p.active
              ? 'min-w-0 flex-1 truncate whitespace-nowrap text-left text-ui-11 text-text cursor-pointer'
              : 'min-w-0 flex-1 truncate whitespace-nowrap text-left text-ui-11 text-text-muted cursor-pointer'}>{tab.title}</button>
        </div>
      ))}
      <div className="flex-none relative">
        <button type="button" onClick={p.onTogglePlus} aria-label="Add panel surface"
          aria-expanded={p.plusOpen} aria-haspopup="menu" data-trigger="right-surface-add"
          className={p.plusOpen
            ? 'size-6 flex-none flex items-center justify-center rounded-md cursor-pointer bg-wash/10'
            : 'size-6 flex-none flex items-center justify-center rounded-md cursor-pointer motion-hover-fade hover:bg-wash/10 active:bg-wash/14 focus:bg-wash/10'}>
          <span className="size-3.5 text-text-muted">{renderIcon('plus')}</span>
        </button>
        {p.plusOpen && (
          <div className="absolute top-6 right-0 w-40">
            <div className="relative motion-menu-in">
              {popoverCard(
                <div className="flex flex-col gap-0.5">
                  {menuRow(false, false, () => p.onAddSurface('terminal'), 'right-plus-terminal', (
                    <>
                      <span className="size-3.5 flex-none text-text-muted">{renderIcon('terminal')}</span>
                      <span>Terminal</span>
                    </>
                  ))}
                  {p.gitDetected && menuRow(false, false, () => p.onAddSurface('diff'), 'right-plus-diff', (
                    <>
                      <span className="size-3.5 flex-none text-text-muted">{renderIcon('git-branch')}</span>
                      <span>Git</span>
                    </>
                  ))}
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export type TitleBarProps = TabStripProps & {
  /// "project @ device" for the selected session. Null on the new-session
  /// canvas, which titles as nothing while keeping the bar's height.
  target: string | null;
  harness: 'claude' | 'openai' | 'none';
  rightPaneOpen: boolean;
  rightPaneExpanded: boolean;
  onToggleRightPane: () => void;
  onToggleExpand: () => void;
};

/// render_session_title_bar: `[harness mark + project @ device] ... [pane
/// header strip] [toggle-changes]`. The title itself was cut in critique
/// round 3 (it already sits in the page-title tier and the session row).
export function renderSessionTitleBar(p: TitleBarProps) {
  const canvas = p.target === null;
  return (
    <header className="h-9.5 flex-none w-full flex flex-row items-center pt-0.5 px-2 gap-2" data-bar="chat-titlebar">
      {!canvas && !(p.rightPaneOpen && p.rightPaneExpanded) && (
        <div className="min-w-0 flex flex-row items-center gap-1.5">
          {p.harness === 'claude' && <span className="size-3.5 flex-none text-claude-brand">{renderIcon('claude-mark')}</span>}
          {p.harness === 'openai' && <span className="size-3.5 flex-none text-text-muted">{renderIcon('openai-mark')}</span>}
          <span className="flex-none text-ui-12 text-text-muted/50">{p.target}</span>
        </div>
      )}
      <div className="flex-1 min-w-0 flex flex-row items-center" />
      {!canvas && p.rightPaneOpen && (
        <div className="flex-1 min-w-0 flex flex-row items-center gap-1 pl-2 pr-1 overflow-hidden">
          {renderRightTabStrip(p)}
          <button type="button" onClick={p.onToggleExpand} data-action="expand-changes"
            aria-label={p.rightPaneExpanded ? 'Collapse changes' : 'Expand changes'}
            className="size-6 flex-none flex items-center justify-center rounded-md text-text-muted cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover">
            <span className="size-4">{renderIcon(rightPaneExpandIcon(p.rightPaneExpanded))}</span>
          </button>
        </div>
      )}
      {!canvas && (
        <button type="button" onClick={p.onToggleRightPane} data-action="toggle-changes"
          aria-label="Toggle changes pane" aria-pressed={p.rightPaneOpen}
          className="size-6 flex-none flex items-center justify-center rounded-md text-text-muted cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover">
          <span className="size-4">{renderIcon('sidebar-minimalistic')}</span>
        </button>
      )}
    </header>
  );
}
