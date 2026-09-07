// Rust: app/crates/ui/src/browser_pane/tabs.rs (strip, one, add_button,
// tooltip_card, TabCard, PlainTooltip).
// Render helpers on the same pane, not second stateful components.
import type { ReactNode } from 'react';
import type { BrowserTab } from '../Browser';
import { renderIcon } from '../../shell/icons';
import { tabTitle } from './state';

// tabs.rs TAB_MAX_WIDTH 168px and TAB_MIN_WIDTH 56px. 56 is min-w-14 exactly;
// 168 has no admitted step, so the strip stops at max-w-40 (160px).
export type TabHandlers = {
  onActivateTab: (id: string) => void;
  onCloseTab: (id: string) => void;
  onOpenTab: () => void;
  onHoverTab: (id: string | null) => void;
};

export function strip(tabs: readonly BrowserTab[], activeId: string | null, handlers: TabHandlers) {
  return (
    <div className="flex-1 min-w-0 flex flex-row items-center gap-0.5 overflow-hidden">
      {tabs.map(tab => one(tab, tab.id === activeId, handlers))}
      {addButton(handlers.onOpenTab)}
    </div>
  );
}

function one(tab: BrowserTab, isActive: boolean, handlers: TabHandlers) {
  const title = tabTitle(tab.title, tab.url);
  return (
    <div key={tab.id} data-tab={tab.id} aria-current={isActive ? 'page' : undefined}
      onMouseEnter={() => handlers.onHoverTab(tab.id)} onMouseLeave={() => handlers.onHoverTab(null)}
      onClick={() => handlers.onActivateTab(tab.id)}
      onAuxClick={() => handlers.onCloseTab(tab.id)}
      className={isActive
        ? 'flex flex-row items-center gap-1.5 h-6 px-2 min-w-14 max-w-40 rounded-md cursor-pointer border border-border bg-surface-raised'
        : 'flex flex-row items-center gap-1.5 h-6 px-2 min-w-14 max-w-40 rounded-md cursor-pointer border border-bg hover:bg-element-hover'}>
      {/* A tab that is still loading says so, without a spinner's cost. */}
      {tab.loading && <span className="size-1.5 flex-none rounded-full bg-busy" />}
      {/* An ellipsis, not a hard clip: titles cut mid-word read as a fault. */}
      <span className={isActive
        ? 'flex-1 min-w-0 truncate text-ui-12 text-text'
        : 'flex-1 min-w-0 truncate text-ui-12 text-text-muted'}>{title}</span>
      <button type="button" aria-label={`Close tab: ${title}`} data-tab-close={tab.id}
        onClick={event => { event.stopPropagation(); handlers.onCloseTab(tab.id); }}
        className="size-4 flex-none flex items-center justify-center rounded-sm cursor-pointer text-text-muted hover:bg-element-active focus:bg-element-active">
        <span className="size-2.5">{renderIcon('close')}</span>
      </button>
    </div>
  );
}

function addButton(onOpenTab: () => void) {
  return (
    <button key="browser-tab-add" type="button" aria-label="New tab" data-tab-add="1" onClick={onOpenTab}
      className="size-6 flex-none flex items-center justify-center rounded-md cursor-pointer text-text-muted hover:bg-element-hover focus:bg-element-hover">
      <span className="size-3">{renderIcon('plus')}</span>
    </button>
  );
}

// The surface every tooltip in this pane sits on. tabs.rs shadow_md() has no
// admitted pair, so the card is held by its border alone.
export function tooltipCard(children: ReactNode) {
  return <div role="tooltip" className="px-2 py-1.5 max-w-80 rounded-md border border-border-strong bg-surface-raised text-ui-11 text-text">{children}</div>;
}

// A few words on hover: the name of a button that shows only an icon.
export function plainTooltip(label: string) {
  return tooltipCard(<span className="whitespace-nowrap">{label}</span>);
}

// A tab's hover card: what the tab is, and where it points. Same surface as
// plainTooltip, two lines instead of one. The address sits under the name,
// quieter, the way Chrome's does.
export function tabCard(title: string, url: string | null) {
  return tooltipCard(
    <span className="flex flex-col gap-0.5">
      <span className="truncate">{title}</span>
      {url !== null && <span className="truncate text-ui-11 text-text-muted">{url}</span>}
    </span>);
}
