// Rust: app/crates/ui/src/terminal/panel.rs (TerminalPanel::render_scrollbar at
// 1256, scrollbar_metrics at 234). An explicit split of the oversized panel.rs.
// Render helper, not a stateful component.

/// The on-demand scrollbar. Native paints it only while the terminal body is
/// hovered and only when the tab has scrollback; the thumb widens from 3px to
/// 4.5px while the rail itself is hovered.
export function renderScrollbar(hovered: boolean, onScrollbarHover: (hovered: boolean) => void) {
  return (
    <div data-terminal="scrollbar"
      onMouseEnter={() => onScrollbarHover(true)}
      onMouseLeave={() => onScrollbarHover(false)}
      className="absolute top-0 bottom-0 right-0 w-2.5 cursor-pointer">
      <div data-terminal="scrollbar-thumb" className={hovered
        ? 'absolute top-12 right-0.5 w-1 h-16 rounded-full bg-text-faint/50'
        : 'absolute top-12 right-0.5 w-0.75 h-16 rounded-full bg-text-faint/50'} />
    </div>
  );
}
