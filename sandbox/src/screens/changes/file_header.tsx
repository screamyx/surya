// Rust: app/crates/ui/src/changes.rs, Changes::render_file_header (the
// FileHeaderPresentation::Row arm) and its chevron.
import type { FileDiff } from './model';

/// Rust: `Changes::render_file_header`'s chevron, changes.rs:3225-3252.
///
/// Glyphs are crate::icons::ALT_ARROW_DOWN / ALT_ARROW_RIGHT, from
/// app/crates/ui/assets/icons. The generated sandbox icon sheet
/// (src/shell/icons.tsx) carries alt-arrow-down but not alt-arrow-right, and
/// that file is generated, so both glyphs are inlined here from the same
/// native assets.
///
/// gpui has no rotation transform at the pinned rev, so the two glyphs
/// crossfade over CHEVRON instead: opacity 0.25 to 1 in 200ms, under a fresh
/// animation id per `fold.epoch`. `epoch` mirrors that id, so a 0 (the first
/// paint, a fold-all, or a fold while wrap is on) paints the rest state with no
/// animation, exactly as `FileFold::animating` gates it.
function chevron(collapsed: boolean, epoch: number) {
  return (
    <div key={epoch} className={epoch > 0
      ? 'flex-none size-3.5 text-text-muted/88 motion-chevron'
      : 'flex-none size-3.5 text-text-muted/88'}>
      {collapsed
        ? (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><path fill="none" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.5" d="m9 5l6 7l-6 7" /></svg>)
        : (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><path fill="none" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.5" d="m19 9l-7 6l-7-6" /></svg>)}
    </div>
  );
}

export type FileHeaderProps = {
  file: FileDiff;
  first: boolean;
  collapsed: boolean;
  /// Rust: `FileFold::epoch`. Bumped by a single-file fold, cleared by a
  /// fold-all and by folding while wrap is on.
  foldEpoch: number;
  onToggleFold: (path: string) => void;
};

/// Chevron, mono path in one quiet tone, then right-aligned +N / -N counts on
/// a slightly raised wash. The header carries the section separator: rows are
/// flat, so there is no per-file wrapper to hang it on.
export function renderFileHeader({ file, first, collapsed, foldEpoch, onToggleFold }: FileHeaderProps) {
  return (
    <button type="button" key={file.path} data-file={file.path}
      aria-expanded={!collapsed} aria-label={`${collapsed ? 'Expand' : 'Collapse'} ${file.path}`}
      onClick={() => onToggleFold(file.path)}
      className={first
        ? 'w-full h-9 flex-none flex flex-row items-center gap-2 px-3 text-left bg-wash/5 cursor-pointer hover:bg-wash/10 active:bg-wash/14 focus:bg-wash/10'
        : 'w-full h-9 flex-none flex flex-row items-center gap-2 px-3 text-left border-t border-border bg-wash/5 cursor-pointer hover:bg-wash/10 active:bg-wash/14 focus:bg-wash/10'}>
      {chevron(collapsed, foldEpoch)}
      <span className="flex-1 min-w-0 truncate whitespace-nowrap text-ui-12 text-text-muted">{file.path}</span>
      {file.binary && <span className="flex-none text-ui-10 text-text-faint">BIN</span>}
      {(file.additions > 0 || !file.binary) && <span className="flex-none text-ui-11 text-diff-add">{`+${file.additions}`}</span>}
      {(file.deletions > 0 || !file.binary) && <span className="flex-none text-ui-11 text-diff-del">{`−${file.deletions}`}</span>}
    </button>
  );
}
