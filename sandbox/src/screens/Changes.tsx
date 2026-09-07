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

  const files = parsed === null ? [] : parsed.files;
  const isCollapsed = (path: string) => collapsed.includes(path);
  const allCollapsed = files.length > 0 && files.every(file => isCollapsed(file.path));

  function toggleFold(path: string) {
    setCollapsed(isCollapsed(path) ? collapsed.filter(entry => entry !== path) : collapsed.concat(path));
  }
  function toggleCollapseAll() {
    setCollapsed(allCollapsed ? [] : files.map(file => file.path));
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
    // Rust: DiffPhase::Preparing. Native spins a gradient loader above the
    // label; no animation class is admitted yet, so the label stands alone.
    <div className="flex-1 flex flex-col items-center justify-center gap-2 text-ui-12 text-text-faint">Preparing diff…</div>
  ) : files.length === 0 ? (
    <div className="flex-1 flex items-center justify-center text-ui-12 text-text-faint">{cleanMessage(scope, baseRef)}</div>
  ) : (
    <div className="flex-1 min-h-0 flex flex-col">
      {renderHeaderStrip(parsed, scope, baseRef)}
      <div className="flex-1 min-h-0 overflow-y-scroll">
        {flattenRows(files, mode, isCollapsed).map(row => (
          <div key={row.key}>{renderRow({
            row, files, mode, wrapped: wrapLines, collapsed: isCollapsed, hover,
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
