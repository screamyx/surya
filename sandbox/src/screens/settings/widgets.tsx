// Rust: app/crates/ui/src/settings/widgets.rs (the shared settings widget kit).
// Plain render helpers, in the same order and under the same names as the Rust
// module. Each returns a node; where the Rust returns a Div the caller fills,
// the React helper takes children. Glyphs come from the generated src/icons.tsx,
// which is built straight from the same app/crates/ui/assets/icons SVGs.
import type { ReactNode } from 'react';
import { renderIcon } from '../../icons';
import type { IconName } from '../../icons';

/// Rust: page_column(). Centered page column, `mx-auto w-full max-w-3xl px-6 pb-16 pt-8`.
export function pageColumn(children: ReactNode) {
  return <div className="w-full max-w-184 mx-auto px-6 pt-8 pb-16 flex flex-col">{children}</div>;
}

/// Rust: page_header(theme, title, count). Title and optional count on one baseline.
export function pageHeader(title: string, count?: number) {
  return (
    <div className="flex flex-row items-baseline gap-2.5">
      <h1 className="text-ui-16 font-semibold text-text">{title}</h1>
      {count !== undefined && <span className="text-ui-13 text-text-faint">{count}</span>}
    </div>
  );
}

/// Rust: page_subtitle(theme, copy). `narrow` is the Notifications page's
/// max_w(512) + line_height(20) on the same helper.
export function pageSubtitle(copy: string, narrow = false) {
  return <p className={narrow
    ? 'mt-1 text-ui-13 text-text-muted max-w-96 leading-normal'
    : 'mt-1 text-ui-13 text-text-muted'}>{copy}</p>;
}

/// Rust: field_label(theme, label). The caption over a picker, not a page headline.
export function fieldLabel(label: string) {
  return <div className="text-ui-13 font-medium text-text">{label}</div>;
}

/// Rust: option_card_row(). A row of equally sized preview cards.
export function optionCardRow(children: ReactNode) {
  return <div className="flex flex-row items-start gap-4 w-full">{children}</div>;
}

/// Rust: option_card(theme, label, selected, preview). The preview frame is the
/// control; the Rust caller adds `.id(..)` and `.on_click(..)`, so `onSelect` is
/// the callback out. The preview must round its own corners to `rounded-md`.
export function optionCard(label: string, selected: boolean, preview: ReactNode, onSelect?: () => void) {
  return (
    <button type="button" onClick={onSelect} aria-pressed={selected} data-option={label}
      className="flex-1 min-w-0 flex flex-col items-center gap-2 cursor-pointer">
      <span className={selected
        ? 'h-40 w-full rounded-md overflow-hidden border border-accent flex'
        : 'h-40 w-full rounded-md overflow-hidden border border-border flex'}>{preview}</span>
      <span className={selected
        ? 'text-ui-13 font-medium text-accent'
        : 'text-ui-13 font-normal text-text-muted'}>{label}</span>
    </button>
  );
}

/// Rust: section_card(theme). The card tone over glass.
export function sectionCard(children: ReactNode) {
  return <div className="mt-6 rounded-xl border border-border bg-surface-card overflow-hidden flex flex-col">{children}</div>;
}

/// Rust: card_row(theme, first). `dimmed` is the Notifications sub-option's
/// `.opacity(0.55)` while its parent toggle is off.
export function cardRow(first: boolean, children: ReactNode, dimmed = false) {
  if (dimmed) {
    return <div className={first
      ? 'px-5 py-3.5 flex flex-row items-center gap-3.5 hover:bg-wash/5 opacity-55'
      : 'px-5 py-3.5 flex flex-row items-center gap-3.5 hover:bg-wash/5 opacity-55 border-t border-border'}>{children}</div>;
  }
  return <div className={first
    ? 'px-5 py-3.5 flex flex-row items-center gap-3.5 hover:bg-wash/5'
    : 'px-5 py-3.5 flex flex-row items-center gap-3.5 hover:bg-wash/5 border-t border-border'}>{children}</div>;
}

/// Rust: row_tile(theme, icon_path). The identity tile around a 16px icon.
export function rowTile(iconPath: IconName) {
  return (
    <div className="flex-none size-9 rounded-lg border border-border bg-wash/5 flex items-center justify-center">
      <span className="size-4 flex text-text-muted">{renderIcon(iconPath)}</span>
    </div>
  );
}

/// Rust: row_title(theme, title). ROW_TITLE_SIZE is 13.
export function rowTitle(title: string) {
  return <div className="min-w-0 truncate text-ui-13 font-medium text-text">{title}</div>;
}

/// Rust: meta_line(theme, fragments). Fragments joined by a quieter dot.
export function metaLine(fragments: readonly ReactNode[]) {
  return (
    <div className="mt-0.25 flex flex-row flex-wrap items-center gap-x-2 gap-y-0.5 text-ui-12 text-text-faint">
      {fragments.flatMap((fragment, index) => index === 0
        ? [<span key={`fragment-${index}`}>{fragment}</span>]
        : [<span key={`dot-${index}`} className="text-text-faint/50">{'·'}</span>,
          <span key={`fragment-${index}`}>{fragment}</span>])}
    </div>
  );
}

/// Rust: badge(theme, label). The right-anchored outline pill.
export function badge(label: string) {
  return <span className="flex-none px-2 py-0.5 rounded-full border border-border text-ui-10 text-text-muted">{label}</span>;
}

/// Rust: badge_active(theme, label). The emerald status pill.
export function badgeActive(label: string) {
  return <span className="flex-none px-2 py-0.5 rounded-full bg-success/14 text-ui-10 text-success-muted/88">{label}</span>;
}

/// Rust: toggle_switch(theme, on). The Rust helper is display-only and the
/// caller adds `.id(..)` and `.on_click(..)`; here the page owns the boolean and
/// passes `onToggle`. Hover, active and focus are sandbox additions - the native
/// pill has none - so a designer can feel the control.
export function toggleSwitch(on: boolean, label: string, onToggle?: () => void, disabled = false) {
  return (
    <button type="button" role="switch" aria-checked={on} aria-label={label} disabled={disabled}
      onClick={onToggle} data-toggle={label}
      className={on
        ? 'flex-none w-8 h-5 rounded-full relative cursor-pointer bg-text hover:bg-text/88 active:bg-text/50 focus:bg-text/88'
        : 'flex-none w-8 h-5 rounded-full relative cursor-pointer bg-wash/10 hover:bg-wash/14 active:bg-wash/50 focus:bg-wash/14'}>
      <span className={on
        ? 'absolute top-0.5 left-3.5 size-4 rounded-full bg-on-solid'
        : 'absolute top-0.5 left-0.5 size-4 rounded-full bg-wash/88'} />
    </button>
  );
}

/// Rust: ghost_action(theme) plus ghost_hover(theme, s), which in Rust is a
/// StyleRefinement the caller applies. Here it is this button's hover, active
/// and focus class list. `muted` is the Servers Remove button's resting
/// `.opacity(0.7)` that its own hover lifts.
export function ghostAction(label: string, iconPath: IconName, onClick?: () => void, muted = false) {
  return (
    <button type="button" onClick={onClick} data-action={label}
      className={muted
        ? 'flex-none flex flex-row items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-ui-12 text-text-muted cursor-pointer opacity-55 hover:opacity-100 hover:bg-wash/5 hover:text-text active:bg-wash/10 focus:opacity-100 focus:bg-wash/5 focus:text-text'
        : 'flex-none flex flex-row items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-ui-12 text-text-muted cursor-pointer hover:bg-wash/5 hover:text-text active:bg-wash/10 focus:bg-wash/5 focus:text-text'}>
      <span className="flex-none size-3.5 flex">{renderIcon(iconPath)}</span>
      {label}
    </button>
  );
}

/// Rust: error_strip(theme, message). Dismissible where the page passes onDismiss.
export function errorStrip(message: string, onDismiss?: () => void) {
  const body = (
    <>
      <span className="flex-none mt-0.5 size-4 flex">{renderIcon('danger-triangle')}</span>
      <span className="min-w-0">{message}</span>
    </>
  );
  if (!onDismiss) return <div role="alert" className="mt-4 px-4 py-3 rounded-xl border border-danger/14 bg-danger/5 text-ui-12 text-danger-muted/88 flex flex-row items-start gap-2">{body}</div>;
  return (
    <button type="button" onClick={onDismiss} aria-label="Dismiss error"
      className="mt-4 px-4 py-3 rounded-xl border border-danger/14 bg-danger/5 text-ui-12 text-danger-muted/88 flex flex-row items-start gap-2 w-full text-left cursor-pointer hover:bg-danger/10 active:bg-danger/14 focus:bg-danger/10">{body}</button>
  );
}

/// Rust: warning_strip(theme, message). The amber sibling of error_strip.
export function warningStrip(message: string) {
  return (
    <div role="alert" className="mt-2 px-4 py-2.5 rounded-xl border border-warning/14 bg-warning/5 text-ui-12 text-warning-muted/88 flex flex-row items-start gap-2">
      <span className="flex-none mt-0.5 size-3.5 flex">{renderIcon('danger-triangle')}</span>
      <span className="min-w-0">{message}</span>
    </div>
  );
}
