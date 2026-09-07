// Rust: app/crates/ui/src/terminal/view.rs (TerminalElement prepaint/paint,
// shape_row, resolve_color) over app/crates/ui/src/terminal/emulator.rs
// (CellSnapshot, CursorSnapshot). Render helper on TerminalPanel, not a second
// stateful component. The emulator itself stays in Rust: this paints a snapshot.

/// Cell foreground role. Native resolves `CellColor` against the theme's own
/// ANSI16 table (`Theme::terminal.ansi`) and its `terminal.foreground`, neither
/// of which the sandbox token set carries. These six roles are the admitted
/// stand-ins; `default` takes `text`, the token nearest native foreground.
export type TermTone = 'default' | 'dim' | 'accent' | 'success' | 'warning' | 'danger';

/// A run of cells sharing one set of attributes, as `shape_row` merges them.
export type TermSpan = { text: string; tone?: TermTone; bold?: boolean; italic?: boolean };

/// One viewport row. `selection` is a half-open column range, matching the
/// merged `sel_quads` runs prepaint builds from `CellSnapshot::selected`.
export type TermRow = { id: string; spans: readonly TermSpan[]; selection?: readonly [number, number] };

/// Cursor in viewport coordinates, row 0 = top of the visible grid.
export type TermCursor = { row: number; col: number };

type TermCell = { ch: string; tone: TermTone; bold: boolean; italic: boolean; selected: boolean; cursor: boolean };
type TermSegment = { text: string; tone: TermTone; bold: boolean; italic: boolean; selected: boolean; cursor: boolean };

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
      out.push({ ch: cursor && ch === ' ' ? '\u00a0' : ch, tone: span.tone ?? 'default', bold: span.bold === true, italic: span.italic === true, selected, cursor });
    }
  }
  if (cursorCol >= 0 && cursorCol >= out.length) {
    for (let col = out.length; col <= cursorCol; col += 1) {
      out.push({ ch: '\u00a0', tone: 'default', bold: false, italic: false, selected: false, cursor: col === cursorCol });
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
    if (last !== undefined && last.tone === cell.tone && last.bold === cell.bold && last.italic === cell.italic
      && last.selected === cell.selected && last.cursor === cell.cursor && !cell.cursor) {
      last.text += cell.ch;
    } else {
      out.push({ text: cell.ch, tone: cell.tone, bold: cell.bold, italic: cell.italic, selected: cell.selected, cursor: cell.cursor });
    }
  }
  return out;
}

/// One merged run. Nested spans carry tone, then face, then the selection wash
/// and the cursor quad, because a class list cannot be built from six booleans.
/// Wrapper order follows the native paint order: backgrounds, selection wash,
/// glyphs, cursor last. The spans stay inline, so a run keeps the space that
/// separates it from the next one.
function renderSegment(segment: TermSegment, index: number, focused: boolean) {
  const glyph = segment.bold || segment.italic
    ? <span className={segment.bold
      ? (segment.italic ? 'font-bold italic' : 'font-bold not-italic')
      : (segment.italic ? 'font-normal italic' : 'font-normal not-italic')}>{segment.text}</span>
    : segment.text;
  const toned = <span className={
    segment.tone === 'dim' ? 'text-text/50'
      : segment.tone === 'accent' ? 'text-accent'
        : segment.tone === 'success' ? 'text-success'
          : segment.tone === 'warning' ? 'text-warning'
            : segment.tone === 'danger' ? 'text-danger'
              : 'text-text'}>{glyph}</span>;
  const washed = segment.selected ? <span className="bg-selection">{toned}</span> : toned;
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
    <div data-terminal="grid" className="size-full overflow-hidden p-3 flex flex-col text-ui-13 leading-normal text-text">
      {rows.map((row, index) => (
        <div key={row.id} data-terminal-row={row.id} className="h-5 flex-none whitespace-nowrap">
          {rowSegments(rowCells(row, cursor !== null && cursor.row === index ? cursor.col : -1))
            .map((segment, ix) => renderSegment(segment, ix, focused))}
        </div>
      ))}
    </div>
  );
}
