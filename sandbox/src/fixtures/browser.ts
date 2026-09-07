// Fixture props for the browser pane. The native engine continues to own the
// CEF process, the tabs and every navigation.
import type { BrowserTab, BrowserPage } from '../screens/Browser';
import type { OffNote } from '../screens/browser/page';

export const seededTabs: readonly BrowserTab[] = [
  { id: 'tab-1', title: 'Example Domain', url: 'https://example.com/', loading: false },
  { id: 'tab-2', title: 'Chromium Embedded Framework docs', url: 'https://docs.cef-project.org/', loading: false },
  { id: 'tab-3', title: '', url: 'https://surya.local/handoff', loading: true },
];

export const seededPage: BrowserPage = {
  url: 'https://example.com/',
  pending: null,
  loading: true,
  canBack: true,
  canForward: false,
  error: null,
  progress: 0.6,
  find: { current: 3, total: 12 },
};

// The zoom the seeded page's host was last read at (state.rs ZoomMemory).
export const seededZoom = 125;

// A fresh window: one blank tab, nothing loaded, no history to step back into.
export const emptyTabs: readonly BrowserTab[] = [
  { id: 'tab-1', title: '', url: '', loading: false },
];

export const emptyPage: BrowserPage = {
  url: '',
  pending: null,
  loading: false,
  canBack: false,
  canForward: false,
  error: null,
  progress: 0,
  find: null,
};

// off.rs note(Off::CacheHeld): the reason a person is most likely to meet.
export const heldNote: OffNote = {
  label: 'Browser off',
  line: 'Another surya window is already using the browser. Close that window, then reopen this one.',
  hint: 'Or set SURYA_CEF_CACHE to a different folder to run both at once.',
};
