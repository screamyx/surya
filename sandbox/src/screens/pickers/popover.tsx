// Rust: app/crates/ui/src/popover.rs - popover_card (295), popover_card_flush
// (316), menu_row (676), menu_row_nav (715), menu_heading (734), tracked_upper
// (746), menu_separator (762), kbd_hint (892), search_input_frame (909),
// menu_section (924), skeleton_rows (1043), skeleton_menu_rows (1087),
// error_row (1117). Plain render helpers, not components.
import type { ReactNode } from 'react';
import { frosted } from './frost';
import { renderIcon } from './icons';
import type { IconName } from './icons';

/// crate::icons::icon(path).size(px(n)) at the sizes this screen paints. The
/// mark takes its paint from the parent's text token through currentColor.
export function icon(name: IconName, size: 11 | 12 | 14 | 16 | 20) {
  return (
    <span aria-hidden="true" className={size === 11 || size === 12
      ? 'flex-none size-3 flex items-center justify-center'
      : size === 14
        ? 'flex-none size-3.5 flex items-center justify-center'
        : size === 16
          ? 'flex-none size-4 flex items-center justify-center'
          : 'flex-none size-5 flex items-center justify-center'}>
      {renderIcon(name)}
    </span>
  );
}

/// popover_card: the floating card. `is_frost()` is false on Windows, so the
/// product path is the opaque `surface_overlay` fill kept here; the macOS and
/// Linux glass tint has no admitted pair (see frost.tsx).
export function popoverCard(children: ReactNode) {
  return frosted(
    <div className="border border-border rounded-xl p-1 overflow-hidden text-ui-13 text-text bg-surface-overlay flex flex-col">
      {children}
    </div>,
  );
}

/// popover_card_flush: the same card without the p-1 inset, for popovers that
/// manage their own internal panes.
export function popoverCardFlush(children: ReactNode) {
  return frosted(
    <div className="border border-border rounded-xl overflow-hidden text-ui-13 text-text bg-surface-overlay flex flex-col">
      {children}
    </div>,
  );
}

/// menu_row_nav: a selected row and the keyboard cursor share one wash, so two
/// selected-looking rows never appear at once.
export function menuRowNav(
  key: string, selected: boolean, highlighted: boolean, onClick: () => void,
  onHover: () => void, children: ReactNode,
) {
  return (
    <button key={key} type="button" data-row={key} onClick={onClick} onMouseEnter={onHover}
      className={selected || highlighted
        ? 'w-full flex flex-row items-center gap-2.5 px-2 py-1.5 rounded-lg text-ui-13 text-left cursor-pointer bg-wash/10 text-text'
        : 'w-full flex flex-row items-center gap-2.5 px-2 py-1.5 rounded-lg text-ui-13 text-left cursor-pointer text-text/88 hover:bg-wash/10 hover:text-text active:bg-element-active focus:bg-wash/10'}>
      {children}
    </button>
  );
}

/// menu_row inside the traits tray: py-1.25 and the tighter rounded-md.
export function traitRow(key: string, active: boolean, onClick: () => void, children: ReactNode) {
  return (
    <button key={key} type="button" data-row={key} onClick={onClick}
      className={active
        ? 'w-full flex flex-row items-center gap-2.5 px-2 py-1 rounded-md text-ui-13 text-left cursor-pointer bg-wash/10 text-text'
        : 'w-full flex flex-row items-center gap-2.5 px-2 py-1 rounded-md text-ui-13 text-left cursor-pointer text-text/88 hover:bg-wash/10 hover:text-text active:bg-element-active focus:bg-wash/10'}>
      {children}
    </button>
  );
}

/// tracked_upper: uppercase with a hair space between letters, the stand-in for
/// the 0.1em tracking gpui has no method for at the pinned revision.
export function trackedUpper(label: string) {
  return label.toUpperCase().split('').join(' ');
}

export function menuHeading(label: string) {
  return <div className="px-2 pb-1 pt-1.5 text-ui-10 font-medium text-text-faint">{trackedUpper(label)}</div>;
}

/// menu_separator: a hairline between sections. The native rule bleeds through
/// the card's p-1 with mx-1 negative margins, which have no admitted pair.
export function menuSeparator(key: string) {
  return <div key={key} className="h-0.25 my-1 flex-none bg-border" />;
}

/// menu_section: a bordered trailing group, edge to edge of the p-1 inset.
export function menuSection(children: ReactNode) {
  return <div className="mt-1 pt-1 border-t border-border flex flex-col gap-0.5">{children}</div>;
}

/// kbd_hint: the model row's jump chip. The native face is the theme's mono
/// family; only font-sans is admitted here.
export function kbdHint(label: string) {
  return <span className="flex-none px-1 py-0.25 rounded-md bg-wash/5 text-ui-10 text-text-faint">{label}</span>;
}

/// search_input_frame with the GPUI-painted field inside it: fixture text (or
/// the placeholder) and a caret block. There is no browser input element; the
/// query is driven by key handling on the popover container.
export function searchBox(query: string, placeholder: string) {
  return (
    <div className="mb-1 px-2.5 py-1.5 rounded-lg bg-wash/5 text-ui-13 flex flex-row items-center gap-0.5">
      {query
        ? <span className="min-w-0 truncate text-text">{query}</span>
        : <span className="min-w-0 truncate text-text-faint">{placeholder}</span>}
      <span className="flex-none w-0.25 h-3.5 bg-caret" />
    </div>
  );
}

/// error_row plus the caller's Retry affordance.
export function errorRow(message: string, onRetry: () => void) {
  return (
    <div className="flex flex-col gap-1.5 p-2 text-ui-12 text-danger">
      <span>{message}</span>
      <button type="button" onClick={onRetry}
        className="px-2 py-0.75 rounded-md border border-border text-text text-left cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover">
        Retry
      </button>
    </div>
  );
}

/// skeleton_rows: full-width ghost slabs. The native bars pulse on SURYA_PULSE;
/// the whitelist admits no animation, so these rest at the wave's mid opacity.
export function skeletonRows(count: number) {
  return (
    <div className="flex flex-col gap-1.5 py-1">
      {Array.from({ length: count }, (unused, i) => (
        <div key={i} className="h-7 rounded-md bg-wash/5 opacity-55" />
      ))}
    </div>
  );
}

/// skeleton_menu_rows: shorter ghost labels of cycling widths, the model
/// picker's loading state. Native widths are fractions of the card (0.42, 0.58,
/// 0.48, 0.66); the whitelist carries fixed steps only.
export function skeletonMenuRows(count: number) {
  return (
    <div className="flex flex-col gap-2 py-1.5 px-1">
      {Array.from({ length: count }, (unused, i) => (
        <div key={i} className={i % 4 === 0
          ? 'h-3.5 w-28 rounded-lg bg-wash/5 opacity-55'
          : i % 4 === 1
            ? 'h-3.5 w-40 rounded-lg bg-wash/5 opacity-55'
            : i % 4 === 2
              ? 'h-3.5 w-32 rounded-lg bg-wash/5 opacity-55'
              : 'h-3.5 w-48 rounded-lg bg-wash/5 opacity-55'} />
      ))}
    </div>
  );
}

/// empty_list_note (pickers.rs:3737): the centered muted note filling an empty
/// model list.
export function emptyListNote(copy: string) {
  return <div className="px-2 py-6 text-ui-12 text-text-faint text-center">{copy}</div>;
}

/// The plain muted note a list-less popover shows instead of rows.
export function popoverNote(copy: string) {
  return <div className="p-2 text-ui-12 text-text-faint">{copy}</div>;
}
