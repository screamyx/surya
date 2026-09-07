// Rust: app/crates/ui/src/popover.rs.
// The shared popover and dialog chrome. The composer and the spaces sidebar
// each grew a private copy during the port and the two disagreed about the same
// Rust file; every helper below is reconciled against popover.rs and the losing
// spelling is named in the comment, so the correction is auditable.
//
// Three approximations carry through from both copies. gpui `shadow_lg` has no
// admitted pair, so a card leans on its hairline alone. The `hairline` and `ink`
// ramps are not admitted either, only `wash`, so a hairline reads as `wash/10`.
// And `hover(|s| s.opacity(0.9))` has no admitted step, so a primary button dims
// its fill to 88 percent instead of the whole element to 90.
import type { ReactNode } from 'react';

/// popover.rs::tracked_upper. gpui has no letter-spacing at the pinned
/// revision, so the Rust interleaves hair spaces; the same trick works here.
export function trackedUpper(label: string) {
  return label.toUpperCase().split('').join(' ');
}

/// popover.rs::popover_card. border_1 + hairline(0.10), CARD_RADIUS is 12.0,
/// p(4.0), overflow_hidden, ui_rems(13), surface_overlay.
/// The spaces copy had rounded-lg and border-border-strong; both are wrong.
/// The helper does not animate: popover_card has no motion call. See menuPopover.
export function popoverCard(children: ReactNode) {
  return (
    <div className="flex flex-col border border-wash/10 rounded-xl p-1 overflow-hidden text-ui-13 text-text bg-surface-overlay">
      {children}
    </div>
  );
}

/// A card at a call site that opens with motion::menu_in. Every popover in both
/// screens does, and the wrapper is separate because menu_in animates a relative
/// inset: on a box that already carries a top-* class it would pin the card to
/// top 0 under its trigger.
export function menuPopover(children: ReactNode) {
  return <div className="relative motion-menu-in">{popoverCard(children)}</div>;
}

/// popover.rs::menu_row and menu_row_nav. gap(10) px(8) py(6) rounded(8).
/// An active row wears card_selected_bg at full text; a resting row sits at
/// text.opacity(0.9) and blends BOTH its text and its fill on hover through
/// motion::hover_blend. The composer copy faded the fill only.
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

/// popover.rs::menu_heading. px(8) pb(4) pt(6), ui_rems(10), MEDIUM,
/// text_muted at 0.6; the nearest admitted alpha is 50.
export function menuHeading(label: string) {
  return <div className="px-2 pb-1 pt-1.5 text-ui-10 font-medium text-text-muted/50">{trackedUpper(label)}</div>;
}

/// popover.rs::menu_separator. h(1) my(4) hairline(0.07). The Rust cancels the
/// card's p(4) inset with mx(-4) to run the hairline border to border; negative
/// margins are not admitted, so this one runs inside the inset.
export function menuSeparator(key: string) {
  return <div key={key} className="h-0.25 my-1 bg-border" />;
}

/// popover.rs::search_input_frame. The query field itself is a GPUI-painted
/// input with no admitted browser pair; the caller paints text and a caret.
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

/// popover.rs::kbd_hint. Native sets font_mono here.
export function kbdHint(label: string) {
  return <span className="flex-none px-1 py-0.25 rounded-sm bg-wash/5 text-ui-10 text-text-muted/50">{label}</span>;
}

/// popover.rs::empty_list_note.
export function emptyNote(label: string) {
  return <div className="px-3 py-2.5 text-ui-12 text-text-muted">{label}</div>;
}

/// The loading band a popup shows before its list lands. Native shimmers it;
/// no admitted animation class covers a moving gradient.
export function skeletonRows(count: number) {
  return (
    <div className="flex flex-col gap-1 p-1">
      {Array.from({ length: count }, (_unused, index) => (
        <div key={index} className="h-8 w-full rounded-lg bg-wash/5" />
      ))}
    </div>
  );
}

/// popover.rs::dialog_card. w(360) p(20) rounded(16) surface_dialog,
/// border hairline(0.10). Neither 360px nor a 16px radius is an admitted step;
/// w-96 is 384 and rounded-xl is 12.
export function dialogCard(children: ReactNode) {
  return <div className="w-96 p-5 rounded-xl bg-surface-dialog border border-wash/10 flex flex-col text-text">{children}</div>;
}

export function dialogTitle(title: string) {
  return <h2 className="text-ui-14 font-semibold text-text">{title}</h2>;
}

export function dialogBody(copy: string) {
  return <p className="text-ui-13 leading-normal text-text-muted">{copy}</p>;
}

/// popover.rs::btn_ghost. px(12) py(6) rounded(8), and hover_blend on BOTH the
/// text (text_muted to text) and the fill (transparent to ink(0.06)).
export function btnGhost(label: string, onClick: () => void) {
  return <button type="button" onClick={onClick}
    className="px-3 py-1.5 rounded-lg text-ui-13 text-text-muted cursor-pointer motion-hover-fade hover:bg-wash/5 hover:text-text active:bg-wash/10 focus:bg-wash/5">{label}</button>;
}

/// popover.rs::btn_primary. px(12) py(6) rounded(8), theme.text fill, on_solid
/// text, and a plain .hover() opacity step, which snaps rather than fading.
/// The composer copy used px-4; the spaces copy used hover:opacity-100, which
/// is a no-op on an already opaque button. `dimmed` is not in btn_primary; the
/// Rust caller that needs it applies its own opacity, so it defaults to off.
export function btnPrimary(label: string, onClick: () => void, dimmed = false) {
  return <button type="button" onClick={onClick}
    className={dimmed
      ? 'px-3 py-1.5 rounded-lg bg-text text-ui-13 font-medium text-on-solid opacity-50 cursor-pointer'
      : 'px-3 py-1.5 rounded-lg bg-text text-ui-13 font-medium text-on-solid cursor-pointer hover:bg-text/88 active:bg-text/50 focus:bg-text/88'}>{label}</button>;
}

/// popover.rs::btn_danger. Same shape on danger_strong.
export function btnDanger(label: string, onClick: () => void) {
  return <button type="button" onClick={onClick}
    className="px-3 py-1.5 rounded-lg bg-danger-strong text-ui-13 font-medium text-on-accent cursor-pointer hover:bg-danger-strong/88 active:bg-danger-strong/50 focus:bg-danger-strong/88">{label}</button>;
}

/// The scrim the dialogs float over, popover::scrim_alpha(0.60). The scrim is
/// static in Rust; motion::dialog_in wraps the card only, so the class rides an
/// inner box and the scrim keeps its own inset.
export function modalScrim(children: ReactNode) {
  return <div className="absolute top-0 left-0 right-0 bottom-0 flex items-center justify-center bg-bg/50">
    <div className="relative motion-dialog-in">{children}</div>
  </div>;
}
