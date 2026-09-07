// Rust: app/crates/ui/src/changes.rs (impl Render for Changes, and
// DiffHeaderTooltip).
import { useState } from 'react';
import type { CommentSide, DiffMode, DiffScope, ParsedDiff } from './changes/model';
import { cleanMessage, flattenRows } from './changes/model';
import { renderHeaderControls } from './changes/controls';
import { renderHeaderStrip } from './changes/header_strip';
import { renderRow } from './changes/rows';

export type ChangesProps = {
  /// null while the diff has not arrived: the Preparing phase.
  parsed: ParsedDiff | null;
  scope: DiffScope;
  /// The comparison ref the branch scope diffs against.
  baseRef: string | null;
  /// A pane-wide failure. Native paints it as a strip above the content.
  error: string | null;
  /// A scoped fetch failure, already translated to its user-facing wording.
  scopedNotice: { message: string; warn: boolean } | null;
  onSelectScope: (scope: DiffScope) => void;
  onAddComment: (path: string, side: CommentSide, line: number) => void;
};

/// Rust: loaders.rs::gradient_spinner, called at changes.rs:4837 with cell_px
/// 3.0 for DiffPhase::Preparing. GSPIN_ROW_TINTS are fixed sunrise hexes with no
/// theme token, so the nearest admitted roles stand in, the same three History
/// uses. A cell's phase is its distance from the wave origin,
/// d = 2 - row + |col - 1|, giving [[3,2,3],[2,1,2],[1,0,1]]: the pulse enters
/// at the bottom edge and converges on the top-centre cell.
function spinnerCell(row: number, col: number) {
  if (row === 0) {
    return <div key={col} className={col === 1
      ? 'size-0.75 rounded-full bg-busy motion-gradient-spin-2'
      : 'size-0.75 rounded-full bg-busy motion-gradient-spin-3'} />;
  }
  if (row === 1) {
    return <div key={col} className={col === 1
      ? 'size-0.75 rounded-full bg-warning motion-gradient-spin-1'
      : 'size-0.75 rounded-full bg-warning motion-gradient-spin-2'} />;
  }
  return <div key={col} className={col === 1
    ? 'size-0.75 rounded-full bg-danger motion-gradient-spin-0'
    : 'size-0.75 rounded-full bg-danger motion-gradient-spin-1'} />;
}

function renderGradientSpinner() {
  return (
    <div aria-hidden="true" className="flex flex-col gap-0.5">
      {[0, 1, 2].map(row => (
        <div key={row} className="flex flex-row gap-0.5">
          {[0, 1, 2].map(col => spinnerCell(row, col))}
        </div>
      ))}
    </div>
  );
}

/// The diff viewer. Fold, layout mode, wrap, the scope menu and line hover
/// are local view state, exactly as they are on the native struct. Everything
/// that reaches git stays a callback.
export function Changes(props: ChangesProps) {
  const { parsed, scope, baseRef, error, scopedNotice } = props;
  const [collapsed, setCollapsed] = useState<readonly string[]>([]);
  const [mode, setMode] = useState<DiffMode>('unified');
  const [wrapLines, setWrapLines] = useState(false);
  const [scopeMenuOpen, setScopeMenuOpen] = useState(false);
  const [tooltip, setTooltip] = useState<string | null>(null);
  const [hover, setHover] = useState<{ path: string; side: CommentSide; line: number } | null>(null);
  // Rust: `FileFold::epoch`, which numbers the chevron's animation id. Only
  // `toggle_fold`'s animating arm bumps it; `toggle_collapse_all` and the
  // wrap-on arm clear `toggled_at`, and the chevron paints its rest state.
  const [foldEpoch, setFoldEpoch] = useState<Readonly<Record<string, number>>>({});

  const files = parsed === null ? [] : parsed.files;
  const isCollapsed = (path: string) => collapsed.includes(path);
  const allCollapsed = files.length > 0 && files.every(file => isCollapsed(file.path));

  function toggleFold(path: string) {
    setCollapsed(isCollapsed(path) ? collapsed.filter(entry => entry !== path) : collapsed.concat(path));
    setFoldEpoch({ ...foldEpoch, [path]: wrapLines ? 0 : (foldEpoch[path] ?? 0) + 1 });
  }
  function toggleCollapseAll() {
    setCollapsed(allCollapsed ? [] : files.map(file => file.path));
    setFoldEpoch({});
  }
  function hoverLine(path: string, anchor: { side: CommentSide; line: number } | null) {
    setHover(anchor === null ? null : { path, side: anchor.side, line: anchor.line });
  }

  // Scoped fetch failures replace the content area. "No turn recorded yet" is
  // the expected pre-first-turn state, not an error.
  const content = scopedNotice !== null ? (
    <div className={scopedNotice.warn
      ? 'flex-1 flex items-center justify-center px-4 text-ui-12 text-warning/88'
      : 'flex-1 flex items-center justify-center px-4 text-ui-12 text-text-faint'}>{scopedNotice.message}</div>
  ) : parsed === null ? (
    // Rust: DiffPhase::Preparing, changes.rs:4830. The gradient loader sits
    // above the label on a SPACE_SM gap.
    <div className="flex-1 flex flex-col items-center justify-center gap-2 text-ui-12 text-text-faint">
      {renderGradientSpinner()}
      <div>Preparing diff…</div>
    </div>
  ) : files.length === 0 ? (
    <div className="flex-1 flex items-center justify-center text-ui-12 text-text-faint">{cleanMessage(scope, baseRef)}</div>
  ) : (
    <div className="flex-1 min-h-0 flex flex-col">
      {renderHeaderStrip(parsed, scope, baseRef)}
      <div className="flex-1 min-h-0 overflow-y-scroll">
        {flattenRows(files, mode, isCollapsed).map(row => (
          <div key={row.key}>{renderRow({
            row, files, mode, wrapped: wrapLines, collapsed: isCollapsed, hover,
            foldEpoch: path => foldEpoch[path] ?? 0,
            onToggleFold: toggleFold, onHoverLine: hoverLine, onAddComment: props.onAddComment,
          })}</div>
        ))}
      </div>
    </div>
  );

  return (
    <section aria-label="Changes" className="size-full flex flex-col">
      {error !== null && (
        <div className="flex-none px-3 py-1 border-b border-border text-ui-11 text-warning">{error}</div>
      )}
      <div className="flex-none">
        {renderHeaderControls({
          scope, mode, wrapLines, allCollapsed, scopeMenuOpen, wrapTooltip: tooltip,
          onToggleScopeMenu: () => setScopeMenuOpen(!scopeMenuOpen),
          onSelectScope: next => { setScopeMenuOpen(false); props.onSelectScope(next); },
          onToggleMode: () => setMode(mode === 'unified' ? 'split' : 'unified'),
          onToggleWrap: () => setWrapLines(!wrapLines),
          onToggleCollapseAll: toggleCollapseAll,
          onHoverControl: setTooltip,
        })}
      </div>
      {content}
    </section>
  );
}
