// Rust: app/crates/ui/src/browser_pane/mod.rs (BrowserPane::render).
import { useState } from 'react';
import { strip, tabCard } from './browser/tabs';
import { row } from './browser/bar';
import { bar as findBar } from './browser/find';
import { offPane, pagePlaceholder, hairline } from './browser/page';
import type { OffNote } from './browser/page';
import { refusal, resolves, tabTooltip, tabTitle } from './browser/state';

// backend.rs Tab: one row in the strip.
export type BrowserTab = { id: string; title: string; url: string; loading: boolean };

// page.rs Page: everything the address bar reads for one tab.
export type BrowserPage = {
  url: string;
  pending: string | null;
  loading: boolean;
  canBack: boolean;
  canForward: boolean;
  error: string | null;
  progress: number;
  find: { current: number; total: number } | null;
};

export type BrowserProps = {
  tabs: readonly BrowserTab[];
  activeTabId: string | null;
  page: BrowserPage;
  // off.rs OffNote, or null while the pane has a page.
  offNote: OffNote | null;
  findOpen: boolean;
  // state.rs ZoomMemory: the active tab's zoom, picked up for the page's host.
  zoomPercent: number;
  // Everything below reaches the CEF process, so it leaves as an intention.
  onNavigate: (typed: string) => void;
  onOpenTab: () => void;
  onCloseTab: (tabId: string) => void;
  onReloadOrStop: () => void;
  onBack: () => void;
  onForward: () => void;
  onFind: (query: string, forward: boolean) => void;
  onZoom: (percent: number) => void;
};

export function BrowserPane(props: BrowserProps) {
  // The address the pane itself wrote into the field, and whether somebody is
  // typing over it. While url_editing is false the field follows the page.
  const shownAddress = props.page.pending ?? props.page.url;
  const [urlEditing, setUrlEditing] = useState(false);
  const [typedUrl, setTypedUrl] = useState(shownAddress);
  const [urlFocused, setUrlFocused] = useState(false);
  // A refusal is the pane's own answer and outlives no navigation, so it wins
  // over the page's last load error while it is showing.
  const [refused, setRefused] = useState<string | null>(null);
  const [activeTabId, setActiveTabId] = useState(props.activeTabId);
  const [findOpen, setFindOpen] = useState(props.findOpen);
  const [findQuery, setFindQuery] = useState('');
  const [findFocused, setFindFocused] = useState(false);
  const [zoomPercent, setZoomPercent] = useState(props.zoomPercent);
  const [hovered, setHovered] = useState<string | null>(null);
  const [hoverTab, setHoverTab] = useState<string | null>(null);

  const url = urlEditing ? typedUrl : shownAddress;
  const error = refused ?? props.page.error;
  const hoveredTab = props.tabs.find(tab => tab.id === hoverTab);
  const card = hoveredTab === undefined ? null
    : tabTooltip(hoveredTab.id === activeTabId, tabTitle(hoveredTab.title, hoveredTab.url), hoveredTab.url);

  // bar.rs submit_url. A scheme the pane refuses keeps the typed text in the
  // field so it can be corrected, and says why: a bar that silently did
  // nothing reads as broken.
  function submitUrl() {
    const typed = url.trim();
    if (typed === '') return;
    if (!resolves(typed)) { setRefused(refusal(typed)); return; }
    setRefused(null);
    setUrlEditing(false);
    props.onNavigate(typed);
  }

  // bar.rs restore_url: escape brings the page's own address back.
  function restoreUrl() {
    setUrlEditing(false);
    setRefused(null);
    setTypedUrl(shownAddress);
  }

  return (
    <section aria-label="Browser" className="relative size-full flex flex-col bg-bg">
      <div className="flex-none h-8 px-1.5 gap-1.5 flex flex-row items-center border-b border-border">
        {strip(props.tabs, activeTabId, {
          onActivateTab: id => { setActiveTabId(id); setUrlEditing(false); setRefused(null); },
          onCloseTab: props.onCloseTab,
          onOpenTab: () => { setUrlEditing(true); setTypedUrl(''); setRefused(null); props.onOpenTab(); },
          onHoverTab: setHoverTab,
        })}
        {findOpen && findBar({
          query: findQuery,
          result: props.page.find,
          focused: findFocused,
          hovered,
          onHover: setHovered,
          onQueryChange: text => { setFindQuery(text); props.onFind(text, true); },
          onFindFocus: setFindFocused,
          onStepFind: forward => props.onFind(findQuery, forward),
          onCloseFind: () => { setFindOpen(false); setFindQuery(''); },
        })}
      </div>
      {row({
        url,
        canBack: props.page.canBack,
        canForward: props.page.canForward,
        loading: props.page.loading,
        zoomPercent,
        urlFocused,
        hovered,
        onHover: setHovered,
        onUrlChange: text => { setUrlEditing(true); setTypedUrl(text); },
        onUrlFocus: setUrlFocused,
        onSubmitUrl: submitUrl,
        onRestoreUrl: restoreUrl,
        onBack: () => { setUrlEditing(false); props.onBack(); },
        onForward: () => { setUrlEditing(false); props.onForward(); },
        onReloadOrStop: props.onReloadOrStop,
        onZoom: percent => { setZoomPercent(percent); props.onZoom(percent); },
      })}
      {hairline(props.page.loading, props.page.progress)}
      {error !== null && <div className="flex-none px-2.5 py-1.5 text-ui-12 text-danger">{error}</div>}
      <div className="flex-1 min-h-0 w-full flex">
        {props.offNote === null ? pagePlaceholder(shownAddress) : offPane(props.offNote)}
      </div>
      {/* gpui draws a tooltip in its own layer, over the address bar. The
          strip is overflow-hidden here, so the card is hoisted to the pane. */}
      {card !== null && <span className="absolute top-8 left-2">{tabCard(card.title, card.url)}</span>}
    </section>
  );
}
