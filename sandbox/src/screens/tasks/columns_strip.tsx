// Rust: app/crates/ui/src/tasks/columns_strip.rs (StripEntry, entries,
// render, max_scroll, render_scroller) and the two board.rs constants it
// agrees with, COLUMN_MIN_W = 220 and COLUMN_GAP = 16.
// The painted edge fades are a linear_gradient; gradients are not admitted,
// so the scroller here has none. See the report's Gaps.
import type { ReactNode } from 'react';
import type { TaskStatus } from './model';
import { COLUMNS, columnLabel } from './model';

/// A card column never squeezes below this; the row scrolls instead.
export const COLUMN_MIN_W = 220;
/// The gap between columns.
export const COLUMN_GAP = 16;

/// One entry in the strip: the column's name, its count, and whether the
/// board is currently showing it.
export type StripEntry = { label: string; count: number; visible: boolean };

/// Which columns the scroller is showing, given the geometry.
export function entries(
  counts: (status: TaskStatus) => number,
  scrolled: number,
  viewportW: number,
  columnW: number,
  gap: number,
): StripEntry[] {
  return COLUMNS.map((status, ix) => {
    const left = ix * (columnW + gap);
    const right = left + columnW;
    return {
      label: columnLabel(status),
      count: counts(status),
      // An UNMEASURED viewport is 0, and 0 would read as "nothing is on
      // screen" - dimming every column on the very first frame, which is the
      // frame this fix exists for.
      visible: viewportW <= 0 || (right > scrolled && left < scrolled + viewportW),
    };
  });
}

/// How far the column row can scroll, WITHOUT waiting for a measurement.
export function maxScroll(measured: number, viewport: number) {
  const content = COLUMNS.length * COLUMN_MIN_W + (COLUMNS.length - 1) * COLUMN_GAP;
  return Math.max(measured, Math.max(content - viewport, 0));
}

/// Draw the strip.
export function renderStrip(strip: readonly StripEntry[]) {
  return (
    <div aria-label="Board columns" className="flex flex-row items-center gap-2.5 px-4 pb-2">
      {strip.map((entry, ix) => (
        <div key={entry.label} data-strip={entry.label} className="flex flex-row items-center gap-1.5">
          {ix > 0 && <span className="pr-1.5 text-ui-12 text-text-faint">{'·'}</span>}
          <span className={entry.visible ? 'text-ui-12 text-text-muted' : 'text-ui-12 text-text-faint'}>{entry.label}</span>
          <span className={entry.visible ? 'text-ui-12 text-text' : 'text-ui-12 text-text-faint'}>{entry.count}</span>
        </div>
      ))}
    </div>
  );
}

/// The columns row: the scroller itself, minus the two painted edge fades.
export function renderScroller(columns: ReactNode) {
  return (
    <div className="relative flex-1 min-w-0 min-h-0 flex">
      <div className="flex-1 min-w-0 flex flex-row gap-4 px-4 pb-4 overflow-x-scroll">{columns}</div>
    </div>
  );
}
