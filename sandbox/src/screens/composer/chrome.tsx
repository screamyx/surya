// Rust: app/crates/ui/src/popover.rs (popover_card, menu_row, kbd_hint,
// btn_ghost, btn_primary, tracked_upper) and badges.rs (render).
import type { ReactNode } from 'react';
import type { IconName } from './icons';
import { renderIcon } from './icons';

// popover.rs::tracked_upper - uppercase with a hair space between letters.
export function trackedUpper(label: string) {
  return label.toUpperCase().split('').join(' ');
}

// popover.rs::popover_card: hairline border, 12px radius, p-1 inset.
// Native adds shadow_lg; CSS shadows have no admitted pair here.
export function popoverCard(children: ReactNode) {
  return (
    <div className="relative flex flex-col border border-wash/10 rounded-xl p-1 overflow-hidden text-ui-13 text-text bg-surface-overlay motion-menu-in">
      {children}
    </div>
  );
}

// popover.rs::menu_row. Active is the selected wash; resting rows brighten on
// hover (motion::hover_blend -> hover:bg-element-hover).
export function menuRow(active: boolean, key: string, onPick: () => void, children: ReactNode) {
  return (
    <button key={key} type="button" data-row={key} onClick={onPick}
      className={active
        ? 'w-full flex flex-row items-center gap-2.5 px-2 py-1.5 rounded-lg text-ui-13 text-left text-text bg-wash/10 cursor-pointer'
        : 'w-full flex flex-row items-center gap-2.5 px-2 py-1.5 rounded-lg text-ui-13 text-left text-text/88 cursor-pointer motion-hover-fade hover:bg-wash/10 active:bg-wash/14 focus:bg-wash/10'}>
      {children}
    </button>
  );
}

// popover.rs::kbd_hint. Native sets font_mono; only font-sans is admitted.
export function kbdHint(label: string) {
  return <span className="flex-none px-1 py-0.25 rounded-sm bg-wash/5 text-ui-10 text-text-muted/50">{label}</span>;
}

// popover.rs::btn_ghost.
export function btnGhost(label: string, onPress: () => void) {
  return <button type="button" onClick={onPress}
    className="px-3 py-1.5 rounded-lg text-ui-13 text-text-muted cursor-pointer motion-hover-fade hover:bg-element-hover active:bg-element-active focus:bg-element-hover">{label}</button>;
}

// popover.rs::btn_primary: white fill, near-black text.
export function btnPrimary(label: string, dimmed: boolean, onPress: () => void) {
  return <button type="button" onClick={onPress}
    className={dimmed
      ? 'px-4 py-1.5 rounded-lg bg-text text-ui-13 font-medium text-on-solid opacity-50 cursor-pointer'
      : 'px-4 py-1.5 rounded-lg bg-text text-ui-13 font-medium text-on-solid cursor-pointer hover:bg-text/88 active:bg-text/50 focus:bg-text/88'}>{label}</button>;
}

// badges.rs::render - the 24px pill the comments chip wears.
export function messageBadge(icon: IconName, label: string) {
  return (
    <span className="h-6 flex flex-row items-center gap-1.5 px-2 rounded-lg bg-wash/5 text-ui-12 font-medium text-text-muted">
      <span className="size-3 flex-none text-text-muted/88">{renderIcon(icon)}</span>
      {label}
    </span>
  );
}

// popover.rs::skeleton_rows stand-in: the loading band a popup shows before
// its list lands. Native animates the shimmer; no admitted animation class.
export function skeletonRows(count: number) {
  return (
    <div className="flex flex-col gap-1 p-1">
      {Array.from({ length: count }, (_unused, index) => (
        <div key={index} className="h-8 w-full rounded-lg bg-wash/5" />
      ))}
    </div>
  );
}

// popover.rs::empty_list_note.
export function emptyNote(label: string) {
  return <div className="px-3 py-2.5 text-ui-12 text-text-muted">{label}</div>;
}
