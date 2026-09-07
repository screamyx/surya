// Rust: app/crates/ui/src/browser_pane/state.rs.
// The pane's arithmetic, with no drawing and no CEF in it. Pure helpers, so
// the same words appear here as on the native screen.

// Chrome's zoom ladder, in percent (state.rs ZOOM_LADDER).
export const ZOOM_LADDER = [25, 33, 50, 67, 75, 80, 90, 100, 110, 125, 150, 175, 200, 250, 300, 400] as const;
export const ZOOM_DEFAULT = 100;

// The ladder step above percent, or the top of the ladder.
export function zoomIn(percent: number): number {
  return ZOOM_LADDER.find(p => p > percent) ?? ZOOM_LADDER[ZOOM_LADDER.length - 1];
}

// The ladder step below percent, or the bottom of the ladder.
export function zoomOut(percent: number): number {
  return [...ZOOM_LADDER].reverse().find(p => p < percent) ?? ZOOM_LADDER[0];
}

// What the bar shows next to the address, or nothing at 100%.
export function zoomLabel(percent: number): string | null {
  return percent === ZOOM_DEFAULT ? null : `${percent}%`;
}

// The key a zoom is remembered against: the host, as Chrome does it.
export function zoomHost(url: string): string | null {
  const rest = url.includes('://') ? url.slice(url.indexOf('://') + 3) : url;
  const head = rest.split(/[/?#]/)[0] ?? '';
  const afterUser = head.includes('@') ? head.slice(head.lastIndexOf('@') + 1) : head;
  const host = afterUser.split(':')[0] ?? '';
  return host === '' ? null : host.toLowerCase();
}

// Why a typed address went nowhere, in words the person can act on. The crate
// takes http and https as written and refuses every other scheme.
export function refusal(typed: string): string {
  const text = typed.trim();
  const cut = text.includes('://') ? text.slice(0, text.indexOf('://')) : text.includes(':') ? text.slice(0, text.indexOf(':')) : '';
  const scheme = cut.toLowerCase();
  if (scheme === '') return 'That address went nowhere. Type a web address or a search.';
  return `${scheme}: addresses do not open here. Only http and https do.`;
}

// Whether a typed address is one the pane will load at all. The crate resolves
// scheme-less input to https and plain words to a search; only another scheme
// comes back empty (backend.rs resolve).
export function resolves(typed: string): boolean {
  const text = typed.trim();
  if (text === '') return false;
  if (!text.includes('://') && !text.includes(':')) return true;
  const scheme = (text.includes('://') ? text.slice(0, text.indexOf('://')) : text.slice(0, text.indexOf(':'))).toLowerCase();
  return scheme === 'http' || scheme === 'https';
}

// What the find field reports: "3 of 12", or "No results".
export function findLabel(current: number, total: number): string {
  if (total <= 0) return 'No results';
  return `${Math.max(current, 1)} of ${total}`;
}

// The words on a tab. A page that has not said its title yet shows its host,
// and a tab with neither reads "New tab".
export function tabTitle(title: string, url: string): string {
  const trimmed = title.trim();
  if (trimmed !== '') return trimmed;
  return zoomHost(url) ?? 'New tab';
}

// The two lines a tab's tooltip shows, when it has anything to show.
export type TabTooltip = { title: string; url: string | null };

// What a tab's tooltip should say, or null for no tooltip at all. The active
// tab says nothing: its title is in the strip and its url is in the bar, and
// gpui anchors the card under the pointer, over the address bar.
export function tabTooltip(isActive: boolean, title: string, url: string): TabTooltip | null {
  if (isActive) return null;
  const name = title.trim();
  const address = url.trim();
  if (name === '') return address === '' ? null : { title: address, url: null };
  return { title: name, url: address !== '' && address !== name ? address : null };
}
