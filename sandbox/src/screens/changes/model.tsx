// Rust: app/crates/ui/src/changes.rs (patch model, DiffRow, flatten_rows,
// file_notices, scope_label, clean_message, gutter_width, split_pairs).
// Pure model helpers only. No JSX and no state; the pane and its row
// helpers read these the way the Rust render path reads the parsed patch.

export type LineKind = 'context' | 'add' | 'del' | 'meta';
export type FileStatus = 'added' | 'deleted' | 'modified' | 'renamed';
export type DiffMode = 'unified' | 'split';
export type DiffPhase = 'preparing' | 'clean' | 'list';
export type CommentSide = 'old' | 'new';

/// Rust: `DiffScope`. `commit` is never listed in the scope menu; a
/// commit-pinned pane is born that way and stays.
export type DiffScope = 'workingTree' | 'branch' | 'latestTurn' | 'history' | 'commit';

export type DiffLine = {
  kind: LineKind;
  /// One-based old-side line number, or null when the line has no old side.
  oldNo: number | null;
  newNo: number | null;
  text: string;
};

export type Hunk = { header: string; lines: readonly DiffLine[] };

export type FileDiff = {
  /// Display path (the post-change side).
  path: string;
  /// Pre-rename path, when different.
  oldPath: string | null;
  status: FileStatus;
  binary: boolean;
  /// Parser-collected notices (mode changes and the like).
  notices: readonly string[];
  hunks: readonly Hunk[];
  additions: number;
  deletions: number;
  /// Largest line number on either side. Sizes the gutters.
  maxLine: number;
};

export type ParsedDiff = {
  truncated: boolean;
  additions: number;
  deletions: number;
  files: readonly FileDiff[];
};

/// Rust: `DiffScope::label`.
export function scopeMenuLabel(scope: DiffScope): string {
  if (scope === 'workingTree') return 'Working tree';
  if (scope === 'branch') return 'Branch changes';
  if (scope === 'latestTurn') return 'Latest turn';
  if (scope === 'history') return 'History';
  return 'Commit';
}

/// Rust: `DiffScope::ALL`. The commit scope is deliberately absent.
export const SCOPE_MENU: readonly DiffScope[] = ['workingTree', 'branch', 'latestTurn', 'history'];

/// Rust: `uncommitted_label`.
export function uncommittedLabel(count: number): string {
  return count === 1 ? '1 Uncommitted change' : `${count} Uncommitted changes`;
}

/// Rust: `scope_label`. The header strip's left-hand label.
export function scopeLabel(scope: DiffScope, count: number, base: string | null): string {
  const files = count === 1 ? 'file' : 'files';
  if (scope === 'workingTree') return uncommittedLabel(count);
  if (scope === 'branch') return base ? `${count} Changed ${files} vs ${base}` : `${count} Changed ${files}`;
  if (scope === 'latestTurn') return `${count} Changed ${files} this turn`;
  if (scope === 'history') return 'History';
  return `${count} Changed ${files} in this commit`;
}

/// Rust: `clean_message`. Empty-state copy per scope.
export function cleanMessage(scope: DiffScope, base: string | null): string {
  if (scope === 'workingTree') return 'No uncommitted changes';
  if (scope === 'branch') return base ? `No changes vs ${base}` : 'No branch changes';
  if (scope === 'latestTurn') return 'No changes this turn';
  if (scope === 'history') return 'No commits found';
  return 'Empty commit';
}

/// Rust: `file_notices`. Status notice first, then binary, then parser notices.
export function fileNotices(file: FileDiff): readonly string[] {
  const notices: string[] = [];
  if (file.status === 'added') notices.push('New file');
  else if (file.status === 'deleted') notices.push('Deleted file');
  else if (file.status === 'renamed') notices.push(`Renamed from ${file.oldPath ?? '?'}`);
  if (file.binary) notices.push('Binary file — contents not shown');
  return notices.concat(file.notices);
}

/// Rust: `gutter_width`, snapped to the admitted width steps. The native
/// formula is `digits * 6.6 + 8 + 6`, never under the classic 36px column.
export type GutterStep = 'w-9' | 'w-10' | 'w-12';
export function gutterStep(file: FileDiff): GutterStep {
  const digits = String(Math.max(file.maxLine, 1)).length;
  const width = Math.max(digits * 6.6 + 14, 36);
  if (width <= 38) return 'w-9';
  if (width <= 44) return 'w-10';
  return 'w-12';
}

/// Rust: `line_anchor`. Which side and line a comment on this line hangs off.
export function lineAnchor(line: DiffLine): { side: CommentSide; line: number } | null {
  if (line.kind === 'del' && line.oldNo !== null) return { side: 'old', line: line.oldNo };
  if (line.kind === 'add' && line.newNo !== null) return { side: 'new', line: line.newNo };
  if (line.kind === 'context' && line.newNo !== null) return { side: 'new', line: line.newNo };
  return null;
}

export type LinePair = { left: number | null; right: number | null };

/// Rust: `split_pairs`. Deletions pair index-wise with the additions that
/// follow them; a context line is mirrored into both columns; the
/// `\ No newline` markers pair with each other and never share a row with
/// code, so either side carrying one makes the whole row a marker row.
export function splitPairs(lines: readonly DiffLine[]): LinePair[] {
  const pairs: LinePair[] = [];
  let dels: number[] = [];
  let adds: number[] = [];
  let delMeta: number[] = [];
  let addMeta: number[] = [];
  function drain(left: number[], right: number[]) {
    for (let ix = 0; ix < Math.max(left.length, right.length); ix += 1) {
      pairs.push({ left: left[ix] ?? null, right: right[ix] ?? null });
    }
  }
  function flush() {
    drain(dels, adds);
    drain(delMeta, addMeta);
    dels = []; adds = []; delMeta = []; addMeta = [];
  }
  lines.forEach((line, ix) => {
    if (line.kind === 'del') {
      if (adds.length > 0 || delMeta.length > 0 || addMeta.length > 0) flush();
      dels.push(ix);
    } else if (line.kind === 'add') {
      if (delMeta.length > 0 || addMeta.length > 0) flush();
      adds.push(ix);
    } else if (line.kind === 'meta') {
      if (adds.length > 0) addMeta.push(ix); else delMeta.push(ix);
    } else {
      flush();
      pairs.push({ left: ix, right: ix });
    }
  });
  flush();
  return pairs;
}

/// Rust: `DiffRow`. The diff flattened so each visible line is its own row.
/// A collapsed file contributes no body rows at all.
export type DiffRow =
  | { row: 'fileHeader'; key: string; file: number }
  | { row: 'notice'; key: string; file: number; notice: number }
  | { row: 'hunkHeader'; key: string; file: number; hunk: number }
  | { row: 'line'; key: string; file: number; hunk: number; line: number }
  | { row: 'splitLine'; key: string; file: number; hunk: number; left: number | null; right: number | null }
  | { row: 'bodyPad'; key: string; file: number };

/// Rust: `body_rows`. Notices, then each hunk header and its lines.
export function bodyRows(fileIx: number, file: FileDiff, mode: DiffMode): DiffRow[] {
  const rows: DiffRow[] = [];
  fileNotices(file).forEach((_, notice) => {
    rows.push({ row: 'notice', key: `n-${fileIx}-${notice}`, file: fileIx, notice });
  });
  file.hunks.forEach((hunk, hunkIx) => {
    rows.push({ row: 'hunkHeader', key: `h-${fileIx}-${hunkIx}`, file: fileIx, hunk: hunkIx });
    if (mode === 'unified') {
      hunk.lines.forEach((_, lineIx) => {
        rows.push({ row: 'line', key: `l-${fileIx}-${hunkIx}-${lineIx}`, file: fileIx, hunk: hunkIx, line: lineIx });
      });
    } else {
      splitPairs(hunk.lines).forEach((pair, pairIx) => {
        rows.push({
          row: 'splitLine', key: `s-${fileIx}-${hunkIx}-${pairIx}`,
          file: fileIx, hunk: hunkIx, left: pair.left, right: pair.right,
        });
      });
    }
  });
  rows.push({ row: 'bodyPad', key: `p-${fileIx}`, file: fileIx });
  return rows;
}

/// Rust: `flatten_rows`. Every file's header, plus its body rows unless the
/// file is collapsed.
export function flattenRows(
  files: readonly FileDiff[],
  mode: DiffMode,
  collapsed: (path: string) => boolean,
): DiffRow[] {
  const rows: DiffRow[] = [];
  files.forEach((file, ix) => {
    rows.push({ row: 'fileHeader', key: `f-${ix}`, file: ix });
    if (!collapsed(file.path)) rows.push(...bodyRows(ix, file, mode));
  });
  return rows;
}
