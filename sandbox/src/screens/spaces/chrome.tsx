// Rust: app/crates/ui/src/popover.rs - popover_card, menu_row, menu_row_nav,
// menu_heading, tracked_upper, menu_separator, search_input_frame,
// dialog_card, dialog_title, dialog_body, dialog_field, btn_ghost,
// btn_primary, btn_danger. Render helpers, not components.
import type { ReactNode } from 'react';

/// popover_card: 1px hairline, 8px radius, p-1 inset, large shadow. gpui
/// shadow_lg has no admitted pair, so the card leans on border-strong.
export function popoverCard(children: ReactNode) {
  return <div className="p-1 rounded-lg border border-border-strong bg-surface-overlay overflow-hidden text-ui-13 text-text">{children}</div>;
}

/// menu_row_nav: a selected row wears the card wash, the keyboard cursor the
/// same lighter one, and never both at once. menu_row installs its
/// motion::hover_blend on neither of those, so only the resting branches fade.
export function menuRow(selected: boolean, highlighted: boolean, onClick: () => void,
  key: string, children: ReactNode, danger?: boolean) {
  const on = selected || highlighted;
  return (
    <button key={key} type="button" onClick={onClick} data-row={key}
      aria-current={selected ? 'true' : undefined}
      className={danger
        ? (on
          ? 'w-full flex flex-row items-center gap-2.5 px-2 py-1.5 rounded-lg text-ui-13 text-left cursor-pointer text-danger bg-wash/10 hover:bg-wash/14 active:bg-wash/14 focus:bg-wash/14'
          : 'w-full flex flex-row items-center gap-2.5 px-2 py-1.5 rounded-lg text-ui-13 text-left cursor-pointer text-danger motion-hover-fade hover:bg-wash/10 active:bg-wash/10 focus:bg-wash/10')
        : (on
          ? 'w-full flex flex-row items-center gap-2.5 px-2 py-1.5 rounded-lg text-ui-13 text-left cursor-pointer text-text bg-wash/10 hover:bg-wash/14 active:bg-wash/14 focus:bg-wash/14'
          : 'w-full flex flex-row items-center gap-2.5 px-2 py-1.5 rounded-lg text-ui-13 text-left cursor-pointer text-text/88 motion-hover-fade hover:bg-wash/10 hover:text-text active:bg-wash/10 focus:bg-wash/10')}>
      {children}
    </button>
  );
}

/// menu_heading. gpui has no letter-spacing at the pinned revision, so
/// tracked_upper interleaves hair spaces; the same trick works here.
export function menuHeading(label: string) {
  return <div className="px-2 pb-1 pt-1.5 text-ui-10 font-medium text-text-muted/50">{trackedUpper(label)}</div>;
}

export function trackedUpper(label: string) {
  return label.toUpperCase().split('').join(' ');
}

/// menu_separator: full-bleed hairline, negative margins cancelling the
/// card's p-1 inset. Negative margins are not admitted, so it runs inside.
export function menuSeparator(key: string) {
  return <div key={key} className="h-0.25 my-1 bg-border" />;
}

/// search_input_frame. The query field itself is a GPUI-painted ComposerInput
/// with no admitted browser pair; the caller paints fixture text and a caret.
export function searchInputFrame(children: ReactNode) {
  return <div className="mb-1 px-2.5 py-1.5 rounded-lg bg-wash/5 text-ui-13">{children}</div>;
}

/// A painted caret standing in for the native input's blinking one.
export function paintedQuery(query: string, placeholder: string) {
  return (
    <span className="flex flex-row items-center gap-0.25">
      {query && <span className="text-text">{query}</span>}
      <span className="w-0.25 h-3.5 bg-caret" />
      {!query && <span className="text-text-faint">{placeholder}</span>}
    </span>
  );
}

export function dialogCard(children: ReactNode) {
  return <div className="w-96 p-5 rounded-xl bg-surface-dialog border border-border-strong flex flex-col text-text">{children}</div>;
}

export function dialogTitle(title: string) {
  return <h2 className="text-ui-14 font-semibold text-text">{title}</h2>;
}

export function dialogBody(copy: string) {
  return <p className="text-ui-13 leading-normal text-text-muted">{copy}</p>;
}

/// btn_ghost blends its text and its wash through motion::hover_blend.
export function btnGhost(label: string, onClick: () => void) {
  return <button type="button" onClick={onClick}
    className="px-3 py-1.5 rounded-lg text-ui-13 text-text-muted cursor-pointer motion-hover-fade hover:bg-wash/5 hover:text-text active:bg-wash/10 focus:bg-wash/5">{label}</button>;
}

/// btn_primary and btn_danger take a plain gpui .hover() opacity step, which
/// snaps. No fade here, and opacity is outside HOVER_FADE's properties anyway.
export function btnPrimary(label: string, onClick: () => void) {
  return <button type="button" onClick={onClick}
    className="px-3 py-1.5 rounded-lg bg-text text-ui-13 font-medium text-on-solid cursor-pointer hover:opacity-100 active:opacity-50 focus:opacity-100">{label}</button>;
}

export function btnDanger(label: string, onClick: () => void) {
  return <button type="button" onClick={onClick}
    className="px-3 py-1.5 rounded-lg bg-danger-strong text-ui-13 font-medium text-on-accent cursor-pointer hover:opacity-100 active:opacity-50 focus:opacity-100">{label}</button>;
}

/// modal: the scrim the dialogs float over. popover::scrim_alpha(0.60). The
/// scrim itself is static in Rust; motion::dialog_in wraps the card only, so
/// the DIALOG_IN class rides an inner box and the scrim keeps its own inset.
export function modalScrim(children: ReactNode) {
  return <div className="absolute top-0 left-0 right-0 bottom-0 flex items-center justify-center bg-bg/50">
    <div className="relative motion-dialog-in">{children}</div>
  </div>;
}
