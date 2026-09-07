// Rust: app/crates/ui/src/changes.rs, Changes::render_row (the virtualized
// list's per-row dispatcher) plus notice_row and hunk_header_row.
import type { CommentSide, DiffMode, DiffRow, FileDiff } from './model';
import { fileNotices, gutterStep, lineAnchor } from './model';
import { renderFileHeader } from './file_header';
import { diffLineRow, metaLineRow, splitFiller, splitLineCell, splitRow } from './lines';

/// Rust: `notice_row`. One "New file" / "Binary file" / truncation note.
export function noticeRow(text: string) {
  return <div className="h-6 w-full flex-none flex items-center px-4 text-ui-11 text-text-faint">{text}</div>;
}

/// Rust: `hunk_header_row`. One `@@ ... @@` row on the bluish-grey wash.
export function hunkHeaderRow(header: string) {
  return <div className="h-7 w-full flex-none flex items-center px-4 bg-diff-hunk-bg text-ui-11 text-text-faint">{header}</div>;
}

export type RowProps = {
  row: DiffRow;
  files: readonly FileDiff[];
  mode: DiffMode;
  wrapped: boolean;
  collapsed: (path: string) => boolean;
  /// Rust: `FileFold::epoch` for this path, 0 when the fold is not animating.
  foldEpoch: (path: string) => number;
  hover: { path: string; side: CommentSide; line: number } | null;
  onToggleFold: (path: string) => void;
  onHoverLine: (path: string, anchor: { side: CommentSide; line: number } | null) => void;
  onAddComment: (path: string, side: CommentSide, line: number) => void;
};

/// One list row. Native materializes only the visible slice; the sandbox
/// renders the whole flattened list inside the same scroller.
export function renderRow(props: RowProps) {
  const { row, files } = props;
  const file = files[row.file];
  if (file === undefined) return null;
  if (row.row === 'fileHeader') {
    return renderFileHeader({
      file, first: row.file === 0, collapsed: props.collapsed(file.path),
      foldEpoch: props.foldEpoch(file.path), onToggleFold: props.onToggleFold,
    });
  }
  if (row.row === 'notice') {
    const text = fileNotices(file)[row.notice];
    return text === undefined ? null : noticeRow(text);
  }
  if (row.row === 'hunkHeader') {
    const hunk = file.hunks[row.hunk];
    return hunk === undefined ? null : hunkHeaderRow(hunk.header);
  }
  if (row.row === 'bodyPad') return <div className="h-2 w-full flex-none" />;
  const hunk = file.hunks[row.hunk];
  if (hunk === undefined) return null;
  const step = gutterStep(file);
  if (row.row === 'line') {
    const line = hunk.lines[row.line];
    if (line === undefined) return null;
    const anchor = lineAnchor(line);
    const hover = props.hover;
    const hovered = anchor !== null && hover !== null
      && hover.path === file.path && hover.side === anchor.side && hover.line === anchor.line;
    return diffLineRow({
      line, path: file.path, step, wrapped: props.wrapped, hovered, anchor,
      onHoverLine: props.onHoverLine, onAddComment: props.onAddComment,
    });
  }
  const left = row.left === null ? undefined : hunk.lines[row.left];
  const right = row.right === null ? undefined : hunk.lines[row.right];
  // `\ No newline at end of file` is a note about the row, not code on one
  // side, so it spans both columns. Pairing never puts a marker opposite
  // code, so either side having one makes the whole row a marker row.
  const meta = [left, right].find(line => line !== undefined && line.kind === 'meta');
  if (meta !== undefined) return metaLineRow(meta.text);
  return splitRow(
    left === undefined ? splitFiller() : splitLineCell(left, left.oldNo, step, props.wrapped),
    right === undefined ? splitFiller() : splitLineCell(right, right.newNo, step, props.wrapped),
    props.wrapped,
  );
}
