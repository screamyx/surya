// Rust: app/crates/ui/src/terminal/view.rs (TerminalElement prepaint/paint,
// shape_row, resolve_color) over app/crates/ui/src/terminal/emulator.rs
// (CellSnapshot, CursorSnapshot). Render helper on TerminalPanel, not a second
// stateful component. The emulator itself stays in Rust: this paints a snapshot.

/// Cell foreground, view.rs:99 resolve_color. `foreground` is
/// `theme.terminal.foreground`; 0 to 15 index `theme.terminal.ansi`. Slots 16
/// to 255 and CellColor::Rgb stay in Rust: extended_indexed_rgb computes them
/// from the index, so they are not theme colours and have no admitted token.
export type TermColor = 'foreground' | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15;

/// A run of cells sharing one set of attributes, as `shape_row` merges them.
/// `dim` is the SGR 2 flag; view.rs:679 multiplies the resolved alpha by 0.6.
export type TermSpan = { text: string; color?: TermColor; bold?: boolean; italic?: boolean; dim?: boolean };

/// One viewport row. `selection` is a half-open column range, matching the
/// merged `sel_quads` runs prepaint builds from `CellSnapshot::selected`.
export type TermRow = { id: string; spans: readonly TermSpan[]; selection?: readonly [number, number] };

/// Cursor in viewport coordinates, row 0 = top of the visible grid.
export type TermCursor = { row: number; col: number };

type TermCell = { ch: string; color: TermColor; bold: boolean; italic: boolean; dim: boolean; selected: boolean; cursor: boolean };
type TermSegment = { text: string; color: TermColor; bold: boolean; italic: boolean; dim: boolean; selected: boolean; cursor: boolean };

/// Expand a row's spans back to cells so selection and the cursor can cut a
/// span in half, the way the native per-cell snapshot already allows.
function rowCells(row: TermRow, cursorCol: number): TermCell[] {
  const out: TermCell[] = [];
  for (const span of row.spans) {
    for (const ch of span.text) {
      const col = out.length;
      const selected = row.selection !== undefined && col >= row.selection[0] && col < row.selection[1];
      const cursor = col === cursorCol;
      // A space under the cursor becomes a no-break space: without
      // whitespace-pre the browser collapses it and the block would vanish.
      out.push({ ch: cursor && ch === ' ' ? '\u00a0' : ch, color: span.color ?? 'foreground', bold: span.bold === true, italic: span.italic === true, dim: span.dim === true, selected, cursor });
    }
  }
  if (cursorCol >= 0 && cursorCol >= out.length) {
    for (let col = out.length; col <= cursorCol; col += 1) {
      out.push({ ch: '\u00a0', color: 'foreground', bold: false, italic: false, dim: false, selected: false, cursor: col === cursorCol });
    }
  }
  return out;
}

/// Merge consecutive cells with identical attributes into one painted run,
/// exactly as `shape_row` merges `TextRun`s and prepaint merges quads.
function rowSegments(cells: readonly TermCell[]): TermSegment[] {
  const out: TermSegment[] = [];
  for (const cell of cells) {
    const last = out.at(-1);
    if (last !== undefined && last.color === cell.color && last.bold === cell.bold && last.italic === cell.italic
      && last.dim === cell.dim && last.selected === cell.selected && last.cursor === cell.cursor && !cell.cursor) {
      last.text += cell.ch;
    } else {
      out.push({ text: cell.ch, color: cell.color, bold: cell.bold, italic: cell.italic, dim: cell.dim, selected: cell.selected, cursor: cell.cursor });
    }
  }
  return out;
}

/// One merged run. Nested spans carry colour, then the dim alpha, then face,
/// then the selection wash and the cursor quad, because a class list cannot be
/// built from seven booleans. Wrapper order follows the native paint order:
/// backgrounds, selection wash, glyphs, cursor last. The spans stay inline, so
/// a run keeps the space that separates it from the next one.
function renderSegment(segment: TermSegment, index: number, focused: boolean) {
  const glyph = segment.bold || segment.italic
    ? <span className={segment.bold
      ? (segment.italic ? 'font-bold italic' : 'font-bold not-italic')
      : (segment.italic ? 'font-normal italic' : 'font-normal not-italic')}>{segment.text}</span>
    : segment.text;
  // view.rs:99 resolve_color, one branch per admitted slot.
  const inked = <span className={
    segment.color === 0 ? 'text-terminal-ansi-0'
      : segment.color === 1 ? 'text-terminal-ansi-1'
        : segment.color === 2 ? 'text-terminal-ansi-2'
          : segment.color === 3 ? 'text-terminal-ansi-3'
            : segment.color === 4 ? 'text-terminal-ansi-4'
              : segment.color === 5 ? 'text-terminal-ansi-5'
                : segment.color === 6 ? 'text-terminal-ansi-6'
                  : segment.color === 7 ? 'text-terminal-ansi-7'
                    : segment.color === 8 ? 'text-terminal-ansi-8'
                      : segment.color === 9 ? 'text-terminal-ansi-9'
                        : segment.color === 10 ? 'text-terminal-ansi-10'
                          : segment.color === 11 ? 'text-terminal-ansi-11'
                            : segment.color === 12 ? 'text-terminal-ansi-12'
                              : segment.color === 13 ? 'text-terminal-ansi-13'
                                : segment.color === 14 ? 'text-terminal-ansi-14'
                                  : segment.color === 15 ? 'text-terminal-ansi-15'
                                    : 'text-terminal-foreground'}>{glyph}</span>;
  // Native scales the resolved alpha by 0.6; 0.55 is the nearest admitted step.
  const toned = segment.dim ? <span className="opacity-55">{inked}</span> : inked;
  const washed = segment.selected ? <span className="bg-terminal-selection">{toned}</span> : toned;
  // The cursor is a full-cell quad in native paint. Focused fills, blurred
  // outlines; the fill is translucent, so the glyph under it stays legible.
  if (segment.cursor) {
    return <span key={index} className={focused ? 'bg-cursor' : 'border border-cursor'}>{washed}</span>;
  }
  return <span key={index}>{washed}</span>;
}

/// The scrollback surface. Native measures cols and rows from the real mono
/// advance and paints quads at `cell_w * col`; here the rows are text and the
/// quads are inline backgrounds on the same merged runs.
export function renderTerminalView(rows: readonly TermRow[], cursor: TermCursor | null, focused: boolean) {
  return (
    <div data-terminal="grid" className="size-full overflow-hidden p-3 flex flex-col text-ui-13 leading-normal text-terminal-foreground">
      {rows.map((row, index) => (
        <div key={row.id} data-terminal-row={row.id} className="h-5 flex-none whitespace-nowrap">
          {rowSegments(rowCells(row, cursor !== null && cursor.row === index ? cursor.col : -1))
            .map((segment, ix) => renderSegment(segment, ix, focused))}
        </div>
      ))}
    </div>
  );
}
