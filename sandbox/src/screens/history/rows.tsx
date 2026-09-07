// Rust: app/crates/ui/src/history.rs, the GitHistory impl render helpers:
// graph_cell (line 861), render_ref (906), render_ref_area (952), render_row (993).
// Render helpers on the same pane, not second stateful components.
import type { ReactNode } from 'react';
import type { HistoryCommit, HistoryGraphRow, HistoryRef, HistoryRefKind, HistorySegmentShape } from '../History';
import { renderIcon } from '../../icons';

// history.rs constants, lines 21 to 34.
const ROW_HEIGHT = 36;
const LANE_SPACING = 12;
const NODE_RADIUS = 3;
const HEAD_RING_PADDING = 2;
const SIDE_PADDING = 5;
const TRAILING_PADDING = 10;
const ROW_OVERLAP = 0.75;
const REF_BADGE_MAX_WIDTH = 112;
const REF_GAP = 5;

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

/// history.rs::format_date. The native "%b %-d, %Y" with a plain dash fallback.
export function formatDate(value: string): string {
  const match = /^(\d{4})-(\d{2})-(\d{2})/.exec(value);
  const month = match ? MONTHS[Number(match[2]) - 1] : undefined;
  if (!match || !month) return '-';
  return `${month} ${Number(match[3])}, ${match[1]}`;
}

/// history.rs::lane_x and ::graph_width.
export function laneX(lane: number) { return SIDE_PADDING + NODE_RADIUS + lane * LANE_SPACING; }
export function graphWidth(laneCount: number) {
  const count = Math.max(laneCount, 1);
  return SIDE_PADDING + TRAILING_PADDING + NODE_RADIUS * 2 + (count - 1) * LANE_SPACING;
}

/// history.rs::estimated_ref_badge_width, ::estimated_ref_overflow_width,
/// ::visible_ref_count. Ported whole: they decide whether the "+N" chip exists,
/// and that chip is the second HistoryRefTooltip trigger.
function badgeWidth(reference: HistoryRef) {
  return Math.min(22 + reference.label.length * 5.7, REF_BADGE_MAX_WIDTH);
}
export function visibleRefCount(refs: readonly HistoryRef[], availableWidth: number) {
  if (refs.length === 0) return 0;
  let visible = 0;
  for (let count = 1; count <= refs.length; count += 1) {
    const hidden = refs.length - count;
    const itemCount = count + (hidden > 0 ? 1 : 0);
    const badges = refs.slice(0, count).reduce((total, ref) => total + badgeWidth(ref), 0);
    const overflow = hidden > 0 ? `+${hidden}`.length * 5.7 : 0;
    if (badges + overflow + Math.max(itemCount - 1, 0) * REF_GAP <= availableWidth) visible = count;
  }
  return visible;
}

/// history.rs::ref_description.
export function refDescription(reference: HistoryRef) {
  const kind = reference.kind === 'branch' ? 'Branch'
    : reference.kind === 'remote' ? 'Remote branch' : 'Tag';
  return `${kind}: ${reference.label}`;
}

function refIcon(kind: HistoryRefKind) {
  return kind === 'branch' ? gitBranchIcon : kind === 'remote' ? renderIcon('cloud') : renderIcon('tag');
}

// git-branch geometry, copied from app/crates/ui/assets/icons/git-branch.svg.
const gitBranchIcon = (
  <svg viewBox="0 0 24 24" fill="none" className="size-full" aria-hidden="true">
    <circle cx="6.5" cy="5.5" r="2.25" stroke="currentColor" strokeWidth="1.5" />
    <circle cx="6.5" cy="18.5" r="2.25" stroke="currentColor" strokeWidth="1.5" />
    <circle cx="17.5" cy="7.5" r="2.25" stroke="currentColor" strokeWidth="1.5" />
    <path d="M6.5 7.75v8.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
    <path d="M17.5 9.75c0 2.9-2.6 4.35-6.2 4.72c-1.9.2-3.3.9-4 2.03" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
  </svg>
);

/// history.rs::graph_cell plus the slice of ::graph_paths that crosses this row.
/// The native paints one canvas behind the whole list; per row is the same
/// picture because segments overlap the row edges by HISTORY_GRAPH_ROW_OVERLAP.
function laneGroup(colorId: number, key: string, children: ReactNode) {
  const index = colorId % 6;
  return (
    <g key={key} className={index === 0 ? 'text-accent' : index === 1 ? 'text-busy'
      : index === 2 ? 'text-success' : index === 3 ? 'text-warning'
        : index === 4 ? 'text-danger' : 'text-text-muted'}>{children}</g>
  );
}

/// history.rs::graph_paths, the three SegmentShape cases. GPUI's
/// cubic_bezier_to(end, c1, c2) becomes SVG's "C c1 c2 end".
function segmentPath(fromLane: number, toLane: number, shape: HistorySegmentShape) {
  const from = laneX(fromLane);
  const to = laneX(toLane);
  const middle = ROW_HEIGHT / 2;
  const top = -ROW_OVERLAP;
  const bottom = ROW_HEIGHT + ROW_OVERLAP;
  if (shape === 'incoming') {
    return `M ${from} ${top} C ${from} ${middle * 0.55} ${to} ${middle * 0.55} ${to} ${middle}`;
  }
  if (shape === 'outgoing') {
    return `M ${from} ${middle} C ${from} ${middle * 1.45} ${to} ${middle * 1.45} ${to} ${bottom}`;
  }
  if (fromLane === toLane) return `M ${from} ${top} L ${to} ${bottom}`;
  return `M ${from} ${top} C ${from} ${middle} ${to} ${middle} ${to} ${bottom}`;
}

export function renderGraphCell(row: HistoryGraphRow, laneCount: number) {
  const width = graphWidth(laneCount);
  const nodeX = laneX(row.nodeLane);
  const middle = ROW_HEIGHT / 2;
  return (
    <div className="relative w-8 h-9 flex-none">
      <svg viewBox={`0 0 ${width} ${ROW_HEIGHT}`} className="size-full" aria-hidden="true">
        {row.segments.map((segment, index) => laneGroup(segment.colorId, `segment-${index}`, (
          <path d={segmentPath(segment.fromLane, segment.toLane, segment.shape)}
            fill="none" stroke="currentColor" strokeWidth="1.5" />
        )))}
        {row.isHead && (
          <g className="text-bg">
            <circle cx={nodeX} cy={middle} r={NODE_RADIUS + HEAD_RING_PADDING - 0.75} fill="currentColor" />
          </g>
        )}
        {laneGroup(row.nodeColorId, 'node', (
          <>
            {row.isHead && (
              <circle cx={nodeX} cy={middle} r={NODE_RADIUS + HEAD_RING_PADDING}
                fill="none" stroke="currentColor" strokeWidth="1" />
            )}
            <circle cx={nodeX} cy={middle} r={NODE_RADIUS} fill="currentColor" />
          </>
        ))}
      </svg>
    </div>
  );
}

/// history.rs::render_ref.
function renderRef(reference: HistoryRef, key: string, handlers: HistoryRowHandlers) {
  const description = refDescription(reference);
  return (
    <span key={key} data-ref={key}
      onMouseEnter={() => handlers.onShowRefs([description])}
      onMouseLeave={() => handlers.onHideRefs()}
      className={reference.kind === 'branch'
        ? 'h-4 max-w-28 px-1.5 flex-none flex items-center gap-0.5 rounded-sm bg-accent/5 text-ui-10 text-accent/88'
        : reference.kind === 'remote'
          ? 'h-4 max-w-28 px-1.5 flex-none flex items-center gap-0.5 rounded-sm bg-busy/5 text-ui-10 text-busy/88'
          : 'h-4 max-w-28 px-1.5 flex-none flex items-center gap-0.5 rounded-sm bg-warning/5 text-ui-10 text-warning/88'}>
      <span aria-hidden="true" className="size-2.5 mt-0.25 flex-none">{refIcon(reference.kind)}</span>
      <span className="min-w-0 truncate">{reference.label}</span>
    </span>
  );
}

/// history.rs::render_ref_area.
function renderRefArea(refs: readonly HistoryRef[], index: number, handlers: HistoryRowHandlers) {
  const visible = visibleRefCount(refs, handlers.refAreaWidth);
  const hidden = refs.slice(visible);
  const descriptions = hidden.map(refDescription);
  return (
    <span className="min-w-0 flex-none overflow-hidden flex items-center gap-1.5">
      {refs.slice(0, visible).map((reference, refIndex) =>
        renderRef(reference, `history-ref-${index}-${refIndex}`, handlers))}
      {hidden.length > 0 && (
        <span data-ref-overflow={index} className="flex-none text-ui-10 text-text-faint"
          onMouseEnter={() => handlers.onShowRefs(descriptions)}
          onMouseLeave={() => handlers.onHideRefs()}>{`+${hidden.length}`}</span>
      )}
    </span>
  );
}

export type HistoryRowHandlers = {
  laneCount: number;
  refAreaWidth: number;
  copiedSha: string | null;
  onOpenCommit: (sha: string) => void;
  onCopySha: (sha: string) => void;
  onShowRefs: (descriptions: readonly string[]) => void;
  onHideRefs: () => void;
};

/// history.rs::render_row, the index < commits.len() branch.
export function renderRow(
  commit: HistoryCommit,
  graphRow: HistoryGraphRow,
  index: number,
  handlers: HistoryRowHandlers,
): ReactNode {
  const copied = handlers.copiedSha === commit.sha;
  const subject = commit.subject === '' ? '(no subject)' : commit.subject;
  return (
    <div key={commit.sha} data-row={commit.sha} role="button" tabIndex={0}
      aria-label={`Open commit: ${subject}`}
      onClick={() => handlers.onOpenCommit(commit.sha)}
      onKeyDown={event => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          handlers.onOpenCommit(commit.sha);
        }
      }}
      className="h-9 w-full flex-none flex flex-row items-center border-b border-wash/5 text-ui-11 cursor-pointer hover:bg-wash/5 focus:bg-wash/5">
      {renderGraphCell(graphRow, handlers.laneCount)}
      <div className="flex-1 min-w-20 overflow-hidden">
        <div className="size-full flex items-center gap-1.5 pr-2">
          <div className="flex-1 min-w-0 truncate text-ui-12 text-text">{subject}</div>
          {commit.refs.length > 0 && renderRefArea(commit.refs, index, handlers)}
        </div>
      </div>
      <div className="w-24 flex-none truncate pr-2 text-text-muted">
        {commit.authorName === '' ? 'Unknown' : commit.authorName}
      </div>
      <div className="w-24 flex-none truncate pr-2 text-ui-10 text-text-muted">
        {formatDate(commit.authoredAt)}
      </div>
      <button type="button" data-sha={commit.sha}
        aria-label={copied ? 'Commit sha copied' : `Copy commit sha ${commit.sha.slice(0, 7)}`}
        onClick={event => { event.stopPropagation(); handlers.onCopySha(commit.sha); }}
        className={copied
          ? 'w-16 h-6 mr-1.5 flex-none flex items-center rounded-sm cursor-pointer text-ui-10 text-accent hover:bg-wash/10 active:bg-wash/14 focus:bg-wash/10'
          : 'w-16 h-6 mr-1.5 flex-none flex items-center rounded-sm cursor-pointer text-ui-10 text-text-muted hover:bg-wash/10 active:bg-wash/14 focus:bg-wash/10'}>
        {copied ? 'Copied' : commit.sha.slice(0, 7)}
      </button>
    </div>
  );
}

/// history.rs::render_row, the index >= commits.len() branch: the pager row.
export function renderLoadOlder(loading: boolean, hasError: boolean, onLoadOlder: () => void): ReactNode {
  const label = loading ? 'Loading…' : hasError ? 'Retry' : 'Load more';
  return (
    <div className="w-full h-12 flex-none flex items-center justify-center">
      <button type="button" data-control="history-load-older" disabled={loading}
        aria-busy={loading} onClick={() => onLoadOlder()}
        className={loading
          ? 'h-7 px-3 flex items-center justify-center gap-1.5 rounded-lg border border-border bg-surface-raised/88 text-ui-11 text-text-faint cursor-default'
          : 'h-7 px-3 flex items-center justify-center gap-1.5 rounded-lg border border-border bg-surface-raised/88 text-ui-11 text-text-muted cursor-pointer hover:bg-element-hover hover:border-border-strong hover:text-text active:bg-element-active focus:bg-element-hover'}>
        {!loading && (
          <span aria-hidden="true" className="size-3 flex-none text-text-faint">
            {hasError ? renderIcon('refresh') : altArrowDownIcon}
          </span>
        )}
        {label}
      </button>
    </div>
  );
}

// alt-arrow-down geometry, copied from app/crates/ui/assets/icons/alt-arrow-down.svg.
const altArrowDownIcon = (
  <svg viewBox="0 0 24 24" className="size-full" aria-hidden="true">
    <path fill="none" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.5" d="m19 9l-7 6l-7-6" />
  </svg>
);
