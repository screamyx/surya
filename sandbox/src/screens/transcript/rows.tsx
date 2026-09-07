// Rust: app/crates/ui/src/transcript.rs (Transcript::render_row, top_gap_for,
// the hover metadata strip and its copy action).
import { icon } from './icons';
import { renderBlock } from './markdown';
import { errorChip, inputChip, permissionChip } from './chips';
import { renderToolGroup } from './tool_group';
import { userAttachments, userBadges, userBubble } from './user';
import { stoppedMark } from './stopped';
import { retryControl } from './retry';
import { workingTrailer, undeliveredRetry } from './working';
import type { TranscriptRow, TranscriptUi } from '../Transcript';

/// Rust: `top_gap_for`. A turn starts on SPACE_LG; two markdown rows of the
/// same part sit MD_BLOCK_GAP apart; anything touching a tool group gets
/// SPACE_MD; everything else SPACE_SM. The first row of the primary transcript
/// also clears the titlebar it fades under.
function topGap(rows: readonly TranscriptRow[], ix: number) {
  const row = rows[ix];
  if (ix === 0) return 'pt-16';
  if (row.turnStart) return 'pt-4';
  const prev = rows[ix - 1];
  const isMd = (r: TranscriptRow) => r.kind.type === 'markdown' || r.kind.type === 'liveMarkdown';
  if (isMd(prev) && isMd(row) && prev.partKey === row.partKey) return 'pt-3';
  if (row.kind.type === 'toolGroup' || prev.kind.type === 'toolGroup') return 'pt-3';
  return 'pt-2';
}

/// Rust: the `match &row.kind` arm of `render_row`.
function rowInner(row: TranscriptRow, ui: TranscriptUi) {
  const kind = row.kind;
  switch (kind.type) {
    case 'user':
      return (
        <div className="w-full flex flex-col">
          {kind.attachments.length > 0 && userAttachments(kind.attachments)}
          {kind.badges.length > 0 && userBadges(kind.badges)}
          {kind.spans.length > 0 && userBubble(kind.spans, kind.pending)}
        </div>
      );
    case 'markdown':
      return renderBlock(kind.block, row.id, ui.code);
    case 'liveMarkdown':
      // Rust wraps this in the per-chunk fade veil; the veil is opacity only,
      // so layout is identical and the next seat applies the motion.
      return <div data-live="1">{renderBlock(kind.block, row.id, ui.code)}</div>;
    case 'toolGroup':
      return renderToolGroup(row.id, kind.tools, kind.autoOpen, kind.summary, ui.toolGroup);
    case 'inputChip':
      return inputChip(kind.header, kind.resolved);
    case 'permissionChip':
      return permissionChip(kind.toolName, kind.command, kind.resolved, kind.allowed, kind.always);
    case 'errorChip':
      return errorChip(kind.message);
  }
}

/// Rust: the hover-revealed metadata strip - a RESERVED lane under the entry's
/// last row. Timestamp, copy action and copied feedback only flip visibility
/// or content, so none of them shifts the virtualizer. User entries align end
/// under the bubble, assistant entries start. Stopped and Retry sit OUTSIDE
/// the fade: they are the answer to "why is this short" and the way to run it
/// again, not cues to hunt for with the pointer.
function metadataStrip(row: TranscriptRow, ui: TranscriptUi) {
  const isUser = row.kind.type === 'user';
  const hovered = ui.hoveredEntry === row.entryId;
  const copied = ui.copiedEntry === row.entryId;
  return (
    <div className={isUser
      ? 'h-8 pt-2 w-full flex items-center justify-end gap-2'
      : 'h-8 pt-2 w-full flex items-center gap-2'}>
      {row.stopped && stoppedMark()}
      {row.retryPrompt !== undefined && retryControl(row.id, row.retryPrompt, ui.onRetryTurn)}
      {hovered && (
        <div data-meta={row.id} className="flex flex-row items-center gap-2">
          <div className="text-ui-12 text-text-muted/50">{row.timestamp}</div>
          {row.copyText !== undefined && (
            <button type="button" data-copy-message={row.entryId} aria-label="Copy message"
              onClick={() => ui.onCopyMessage(row.entryId, row.copyText ?? '')}
              className="size-6 flex items-center justify-center rounded-md cursor-pointer text-text-muted hover:bg-wash/10 active:bg-element-active focus:bg-wash/10">
              <span className="size-3.5 flex items-center justify-center">{copied ? icon('check') : icon('copy')}</span>
            </button>
          )}
        </div>
      )}
    </div>
  );
}

/// Rust: `Transcript::render_row`. Wide gutters around the 46rem column, the
/// computed turn gap on top, and the last row's clearance below. Hovering
/// anywhere on the entry reveals its strip, and only the row that OWNS the
/// reveal may clear it - a stale leave from an earlier row must not blank the
/// strip the newly entered row just lit.
export function renderRow(
  rows: readonly TranscriptRow[],
  ix: number,
  ui: TranscriptUi,
) {
  const row = rows[ix];
  const last = ix + 1 === rows.length;
  const gap = topGap(rows, ix);
  // The lint takes literal class lists or ternaries of them, so the gap and
  // the last row's composer clearance are spelled out rather than joined.
  return (
    <div key={row.id} data-row={row.id}
      onMouseEnter={() => ui.onHoverEntry(row.id, row.entryId)}
      onMouseLeave={() => ui.onHoverEntry(row.id, null)}
      className={gap === 'pt-16'
        ? (last ? 'w-full flex justify-center px-12 pt-16 pb-8' : 'w-full flex justify-center px-12 pt-16')
        : gap === 'pt-4'
          ? (last ? 'w-full flex justify-center px-12 pt-4 pb-8' : 'w-full flex justify-center px-12 pt-4')
          : gap === 'pt-3'
            ? (last ? 'w-full flex justify-center px-12 pt-3 pb-8' : 'w-full flex justify-center px-12 pt-3')
            : (last ? 'w-full flex justify-center px-12 pt-2 pb-8' : 'w-full flex justify-center px-12 pt-2')}>
      <div className="w-full max-w-184 min-w-0">
        {rowInner(row, ui)}
        {row.timestamp !== undefined && metadataStrip(row, ui)}
        {last && ui.undelivered && undeliveredRetry(ui.onRetrySend)}
        {last && !ui.undelivered && ui.working !== null && workingTrailer(ui.working)}
      </div>
    </div>
  );
}
