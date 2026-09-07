// Rust: app/crates/ui/src/loaders.rs (gradient_spinner, mini_glyph_spinner),
// over the shared cell counts in app/crates/proto/src/motion.rs.
// The motion itself is the next seat's job: today's whitelist admits no
// animation class, so every cell here rests at its dim value.

/// Rust: `gradient_spinner` - the WorkingIndicator's 3x3 matrix. Each cell
/// pulses opacity once per GRADIENT_SPIN period (750ms) and the wave enters
/// at the bottom edge and converges on the top-centre cell, so it reads as
/// travelling upward. `GSPIN_ROW_TINTS` paints row 0 blue, row 1 amber, row 2
/// pink; those are literal hexes in surya_proto, not theme tokens, so all
/// nine cells rest on one admitted token here.
export function gradientSpinner() {
  return (
    <div data-loader="gradient-spin" className="flex flex-col gap-0.25">
      {[0, 1, 2].map(row => (
        <div key={row} className="flex flex-row gap-0.25">
          {[0, 1, 2].map(col => (
            <div key={col} data-gspin-cell={`${row}-${col}`} className="size-0.5 rounded-full bg-text-muted" />
          ))}
        </div>
      ))}
    </div>
  );
}

/// Rust: `mini_glyph_spinner` - the 2x3 activity glyph in a running spawn
/// chip's trailing slot. Brightness chases clockwise around the perimeter on
/// the same GRADIENT_SPIN period.
export function miniGlyphSpinner() {
  return (
    <div data-loader="mini-glyph" className="flex flex-col gap-0.25">
      {[0, 1, 2].map(row => (
        <div key={row} className="flex flex-row gap-0.25">
          {[0, 1].map(col => (
            <div key={col} data-mini-cell={`${row}-${col}`} className="size-0.75 rounded-full bg-text-muted" />
          ))}
        </div>
      ))}
    </div>
  );
}
