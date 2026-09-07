// Rust: app/crates/ui/src/history.rs (GitHistory::render, line 1190).
// The pager row, the commit row, the ref badges, the ref area and the graph cell
// are impl render helpers and stay in ./history/rows.tsx.
import { useState } from 'react';
import type { ReactNode } from 'react';
import { renderLoadOlder, renderRow } from './history/rows';
import { HistoryRefTooltip } from './history/ref_tooltip';

// surya_proto::entities, GitHistoryRefKind / GitHistoryRef / GitHistoryCommit.
export type HistoryRefKind = 'branch' | 'remote' | 'tag';
export type HistoryRef = { kind: HistoryRefKind; label: string };
export type HistoryCommit = {
  sha: string;
  parentShas: readonly string[];
  subject: string;
  authorName: string;
  authorEmail: string;
  authoredAt: string;
  refs: readonly HistoryRef[];
};

// history.rs, the private GraphLayout the pane derives from the commit page.
export type HistorySegmentShape = 'through' | 'incoming' | 'outgoing';
export type HistorySegment = {
  fromLane: number; toLane: number; colorId: number; shape: HistorySegmentShape;
};
export type HistoryGraphRow = {
  sha: string;
  nodeLane: number;
  nodeColorId: number;
  segments: readonly HistorySegment[];
  isHead: boolean;
};
export type HistoryGraph = { rows: readonly HistoryGraphRow[]; maxLaneCount: number };

export type GitHistoryProps = {
  /// false stands for the native target_key being None.
  hasRepository: boolean;
  commits: readonly HistoryCommit[];
  graph: HistoryGraph;
  /// next_cursor.is_some(): the list carries one extra pager row when true.
  hasMore: boolean;
  loading: boolean;
  error: string | null;
  fetchError: string | null;
  copiedSha: string | null;
  /// container_query() measures this natively. Supplied as a fixture prop here
  /// because measurement needs a ref, which this sandbox does not admit.
  refAreaWidth: number;
  onOpenCommit: (sha: string) => void;
  onCopySha: (sha: string) => void;
  onLoadOlder: () => void;
};

function renderBanner(message: string): ReactNode {
  return (
    <div className="h-7 flex-none flex items-center px-2 border-b border-danger/14 bg-danger/5 truncate text-ui-11 text-danger-muted">
      {message}
    </div>
  );
}

// loaders.rs::gradient_spinner (history.rs:1214, cell_px 3.0). Its GSPIN_ROW_TINTS
// are fixed sunrise hexes with no theme token; the nearest admitted roles stand in.
// Each cell's phase is its distance from the wave origin, d = 2 - row + |col - 1|,
// so the grid is [[3,2,3],[2,1,2],[1,0,1]] and the pulse travels upward.
function spinnerCell(row: number, col: number): ReactNode {
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

function renderGradientSpinner(): ReactNode {
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

export function GitHistory(props: GitHistoryProps) {
  const { commits, graph, error, fetchError, loading } = props;
  const [refTooltip, setRefTooltip] = useState<readonly string[]>([]);
  const handlers = {
    laneCount: graph.maxLaneCount,
    refAreaWidth: props.refAreaWidth,
    copiedSha: props.copiedSha,
    onOpenCommit: props.onOpenCommit,
    onCopySha: props.onCopySha,
    onShowRefs: (descriptions: readonly string[]) => setRefTooltip(descriptions),
    onHideRefs: () => setRefTooltip([]),
  };

  let body: ReactNode;
  if (!props.hasRepository) {
    body = (
      <div className="flex-1 flex items-center justify-center text-ui-12 text-text-faint">
        No repository selected
      </div>
    );
  } else if (loading && commits.length === 0) {
    body = (
      <div className="flex-1 flex flex-col items-center justify-center gap-2">
        {renderGradientSpinner()}
        <div className="text-ui-12 text-text-faint">Loading history…</div>
      </div>
    );
  } else if (commits.length === 0) {
    body = (
      <div className={error !== null
        ? 'flex-1 flex items-center justify-center px-5 text-ui-12 text-warning'
        : 'flex-1 flex items-center justify-center px-5 text-ui-12 text-text-faint'}>
        {error ?? 'No commits found'}
      </div>
    );
  } else {
    body = (
      <div className="relative flex-1 min-h-0">
        <div className="size-full overflow-y-scroll">
          {commits.map((commit, index) => {
            const graphRow = graph.rows[index];
            return graphRow ? renderRow(commit, graphRow, index, handlers) : null;
          })}
          {props.hasMore && renderLoadOlder(loading, error !== null, props.onLoadOlder)}
        </div>
        {refTooltip.length > 0 && (
          <div className="absolute bottom-2 right-2">
            <HistoryRefTooltip descriptions={refTooltip} />
          </div>
        )}
      </div>
    );
  }

  return (
    <section aria-label="Git history" className="size-full flex flex-col">
      {fetchError !== null && renderBanner(`Fetch failed: ${fetchError}`)}
      {error !== null && commits.length > 0 && renderBanner(error)}
      {commits.length > 0 && (
        <div className="h-6 flex-none flex items-center border-b border-wash/5 text-ui-10 text-text-faint">
          <div className="w-8 flex-none" />
          <div className="flex-1 min-w-20">Commit</div>
          <div className="w-24 flex-none">Author</div>
          <div className="w-24 flex-none">Date</div>
          <div className="w-16 mr-1.5 flex-none">SHA</div>
        </div>
      )}
      {body}
    </section>
  );
}
