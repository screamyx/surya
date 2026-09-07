// Rust: app/crates/ui/src/loaders.rs (gradient_spinner, mini_glyph_spinner),
// over the shared cell counts and phase math in app/crates/proto/src/motion.rs.

/// Rust: `gradient_spinner`'s per-cell phase. `d = MATRIX_SIDE - 1 - row +
/// |col - centre|` over a 3x3 grid, so the grid reads
/// `[[3,2,3],[2,1,2],[1,0,1]]` and the wave enters at the bottom edge and
/// converges on the top-centre cell. `motion-gradient-spin-{d}` carries that
/// same `d / (max + 1)` offset as a negative animation-delay.
function gspinCell(row: number, col: number) {
  const d = 2 - row + Math.abs(col - 1);
  return (
    <div key={col} data-gspin-cell={`${row}-${col}`}
      className={d === 0
        ? 'size-0.5 rounded-full bg-text-muted motion-gradient-spin-0'
        : d === 1
          ? 'size-0.5 rounded-full bg-text-muted motion-gradient-spin-1'
          : d === 2
            ? 'size-0.5 rounded-full bg-text-muted motion-gradient-spin-2'
            : 'size-0.5 rounded-full bg-text-muted motion-gradient-spin-3'} />
  );
}

/// Rust: `gradient_spinner` - the WorkingIndicator's 3x3 matrix. Each cell
/// pulses opacity 1 to 0.1 and back once per GRADIENT_SPIN period (750ms).
/// `GSPIN_ROW_TINTS` paints row 0 blue, row 1 amber, row 2 pink; those are
/// literal hexes in surya_proto, not theme tokens, so all nine cells rest on
/// one admitted token here.
export function gradientSpinner() {
  return (
    <div data-loader="gradient-spin" className="flex flex-col gap-0.25">
      {[0, 1, 2].map(row => (
        <div key={row} className="flex flex-row gap-0.25">
          {[0, 1, 2].map(col => gspinCell(row, col))}
        </div>
      ))}
    </div>
  );
}

/// Rust: `mini_glyph_spinner` - the 2x3 activity glyph in a running spawn
/// chip's trailing slot. `transcript.rs:6135` passes `theme.glyph`, and
/// `GlyphPalette::rows()` is `[light, mid, deep]`, one per row.
///
/// Brightness chases clockwise around the perimeter on the same GRADIENT_SPIN
/// period, but its phase is the ring position over 6
/// (`RING = [[0,1],[5,2],[4,3]]`), so the six cells need 0, 1/6, 2/6, 3/6, 4/6
/// and 5/6. The catalog carries the gradient spinner's four phases only
/// (0, 1/4, 2/4, 3/4). Four of the six have no admitted class, and a partial
/// set would chase in the wrong order, so the cells rest.
export function miniGlyphSpinner() {
  return (
    <div data-loader="mini-glyph" className="flex flex-col gap-0.25">
      {[0, 1, 2].map(row => (
        <div key={row} className="flex flex-row gap-0.25">
          {[0, 1].map(col => (
            <div key={col} data-mini-cell={`${row}-${col}`}
              className={row === 0
                ? 'size-0.75 rounded-full bg-glyph-light'
                : row === 1
                  ? 'size-0.75 rounded-full bg-glyph-mid'
                  : 'size-0.75 rounded-full bg-glyph-deep'} />
          ))}
        </div>
      ))}
    </div>
  );
}
