// Rust: app/crates/ui/src/history.rs (GitHistoryFetchButton::render, line 378).
// Hosted by the Changes surface strip (changes.rs), not by GitHistory itself.
// The busy glyph is loaders.rs::mini_glyph_spinner painted at its static end state:
// see the report's Motion map for the animation a later seat applies.
import { cloudIcon } from './icons';

// mini_glyph_spinner's 2x3 grid. Its tints are theme.glyph, a GlyphPalette derived
// from the accent, which gen-theme.mjs does not emit as a scalar token.
function miniCell(row: number) {
  return <div className={row === 2
    ? 'size-0.5 rounded-full bg-accent-strong'
    : 'size-0.5 rounded-full bg-accent'} />;
}

function renderBusyGlyph() {
  return (
    <span aria-hidden="true" className="flex-none flex flex-col gap-0.25">
      {[0, 1, 2].map(row => (
        <span key={row} className="flex flex-row gap-0.25">{miniCell(row)}{miniCell(row)}</span>
      ))}
    </span>
  );
}

export type GitHistoryFetchButtonProps = { fetching: boolean; onFetchAll: () => void };
export function GitHistoryFetchButton({ fetching, onFetchAll }: GitHistoryFetchButtonProps) {
  return (
    <button type="button" data-control="history-fetch-all" disabled={fetching}
      aria-busy={fetching} aria-label={fetching ? 'Fetching all refs' : 'Fetch all refs'}
      onClick={() => onFetchAll()}
      className={fetching
        ? 'h-6 px-2 flex-none flex items-center justify-center gap-1.5 rounded-md bg-wash/5 cursor-default'
        : 'h-6 px-2 flex-none flex items-center justify-center gap-1.5 rounded-md cursor-pointer hover:bg-wash/14 active:bg-wash/10 focus:bg-wash/14'}>
      {fetching ? renderBusyGlyph() : (
        <span aria-hidden="true" className="size-3 flex-none text-text-muted/88">{cloudIcon}</span>
      )}
      <span className={fetching
        ? 'whitespace-nowrap text-ui-11 text-text-faint'
        : 'whitespace-nowrap text-ui-11 text-text-muted'}>{fetching ? 'Fetching…' : 'Fetch all'}</span>
    </button>
  );
}
