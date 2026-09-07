// Rust: app/crates/ui/src/changes.rs (diff_line_row, meta_line_row,
// code_text_viewport, split_line_cell, split_filler, split_row,
// render_comment_adder, positioned_adder).
// Render helpers on the Changes pane, not second stateful components.
import type { ReactNode } from 'react';
import type { DiffLine, GutterStep, CommentSide } from './model';

/// The line-number gutter. Native paints width and tone on one div; the
/// admitted class vocabulary needs literal lists, so the width sits on the
/// column and the tone on the number it holds.
function gutterCell(no: number | null, kind: DiffLine['kind'], own: boolean, step: GutterStep) {
  return (
    <div className={step === 'w-9' ? 'w-9 flex-none flex justify-end pr-2'
      : step === 'w-10' ? 'w-10 flex-none flex justify-end pr-2'
        : 'w-12 flex-none flex justify-end pr-2'}>
      <span className={own && kind === 'add' ? 'text-ui-11 leading-gpui text-diff-add/88'
        : own && kind === 'del' ? 'text-ui-11 leading-gpui text-diff-del/88'
          : 'text-ui-11 leading-gpui text-text-faint/88'}>{no === null ? '' : String(no)}</span>
    </div>
  );
}

/// Accent bar: solid colour on +/- rows, an invisible spacer on context rows
/// so the columns always align.
function accentBar(kind: DiffLine['kind']) {
  return <div className={kind === 'add' ? 'w-0.75 self-stretch flex-none bg-diff-add/50'
    : kind === 'del' ? 'w-0.75 self-stretch flex-none bg-diff-del/50'
      : 'w-0.75 self-stretch flex-none'} />;
}

function markerCell(kind: DiffLine['kind'], split: boolean) {
  const glyph = kind === 'add' ? '+' : kind === 'del' ? '−' : '·';
  return (
    <div className={split ? 'w-4 flex-none flex justify-center' : 'w-7 flex-none flex justify-center'}>
      <span className={kind === 'add' ? 'text-ui-12 leading-gpui text-diff-add'
        : kind === 'del' ? 'text-ui-12 leading-gpui text-diff-del'
          : 'text-ui-12 leading-gpui text-text-faint/50'}>{glyph}</span>
    </div>
  );
}

/// Rust: `code_text_viewport`. The only part of a row allowed to exceed its
/// viewport. The outer element keeps row chrome fixed; the inner element owns
/// the intrinsic code width. Native shares one horizontal scroll handle
/// across every row; the sandbox clips instead (see the report's gaps).
function codeViewport(text: string, wrapped: boolean, split: boolean) {
  return (
    <div className="flex-1 min-w-0 min-h-5 overflow-hidden">
      <div className={wrapped
        ? (split ? 'w-full min-w-0 pl-1.5 text-ui-12 leading-gpui text-text/88 whitespace-normal'
          : 'w-full min-w-0 pl-3 text-ui-12 leading-gpui text-text/88 whitespace-normal')
        : (split ? 'pl-1.5 text-ui-12 leading-gpui text-text/88 whitespace-nowrap'
          : 'pl-3 text-ui-12 leading-gpui text-text/88 whitespace-nowrap')}>{text}</div>
    </div>
  );
}

/// Rust: `meta_line_row`. `\ No newline at end of file` and friends: a note
/// about the row rather than code, so it is indented past the columns and
/// never tinted.
export function metaLineRow(text: string) {
  return (
    <div className="h-5 w-full flex-none flex items-center pl-20 text-ui-10 text-text-faint italic">{text}</div>
  );
}

/// Rust: `render_comment_adder` inside `positioned_adder`. Appears on the
/// hovered line only. Everything past the click is the engine's, so the
/// sandbox stops at the callback.
function commentAdder(path: string, side: CommentSide, line: number, step: GutterStep,
  onAddComment: (path: string, side: CommentSide, line: number) => void) {
  return (
    <div className={side === 'old' ? 'absolute left-3.5 top-0 h-full flex items-center'
      : step === 'w-9' ? 'absolute left-12 top-0 h-full flex items-center'
        : step === 'w-10' ? 'absolute left-14 top-0 h-full flex items-center'
          : 'absolute left-16 top-0 h-full flex items-center'}>
      <button type="button" data-adder={`${path}:${side}:${line}`}
        aria-label={`Comment on ${path} line ${line}`}
        onClick={() => onAddComment(path, side, line)}
        className="size-4 flex items-center justify-center rounded-sm bg-solid text-on-solid cursor-pointer hover:bg-accent active:bg-accent-strong focus:bg-accent">
        <span className="text-ui-10 leading-none">+</span>
      </button>
    </div>
  );
}

export type DiffLineRowProps = {
  line: DiffLine;
  path: string;
  step: GutterStep;
  wrapped: boolean;
  hovered: boolean;
  anchor: { side: CommentSide; line: number } | null;
  onHoverLine: (path: string, anchor: { side: CommentSide; line: number } | null) => void;
  onAddComment: (path: string, side: CommentSide, line: number) => void;
};

/// Rust: `diff_line_row`. Coloured accent bar, dual line-number gutters,
/// marker column, then the code. Row tints are the reference's ~5% washes.
export function diffLineRow(props: DiffLineRowProps) {
  const { line, path, step, wrapped, hovered, anchor } = props;
  if (line.kind === 'meta') return metaLineRow(line.text);
  const row = (
    <div className={line.kind === 'add'
      ? (wrapped ? 'min-h-5 w-full flex-none flex flex-row items-start bg-diff-add/5' : 'h-5 w-full flex-none flex flex-row items-start bg-diff-add/5')
      : line.kind === 'del'
        ? (wrapped ? 'min-h-5 w-full flex-none flex flex-row items-start bg-diff-del/5' : 'h-5 w-full flex-none flex flex-row items-start bg-diff-del/5')
        : (wrapped ? 'min-h-5 w-full flex-none flex flex-row items-start' : 'h-5 w-full flex-none flex flex-row items-start')}>
      {accentBar(line.kind)}
      {gutterCell(line.oldNo, line.kind, line.kind === 'del', step)}
      {gutterCell(line.newNo, line.kind, line.kind === 'add', step)}
      {markerCell(line.kind, false)}
      {codeViewport(line.text, wrapped, false)}
    </div>
  );
  if (anchor === null) return row;
  return (
    <div className={hovered ? 'w-full relative' : 'w-full'} data-line={`${path}:${anchor.side}:${anchor.line}`}
      onMouseEnter={() => props.onHoverLine(path, anchor)}
      onMouseLeave={() => props.onHoverLine(path, null)}>
      {row}
      {hovered && commentAdder(path, anchor.side, anchor.line, step, props.onAddComment)}
    </div>
  );
}

/// Rust: `split_line_cell`. One half of a split row: the same accent bar,
/// gutter, marker and code columns a unified row uses, minus the second
/// gutter. Each half numbers only its own side.
export function splitLineCell(line: DiffLine, number: number | null, step: GutterStep, wrapped: boolean) {
  return (
    <div className={line.kind === 'add' ? 'flex-1 min-w-0 self-stretch overflow-hidden flex flex-row items-start bg-diff-add/5'
      : line.kind === 'del' ? 'flex-1 min-w-0 self-stretch overflow-hidden flex flex-row items-start bg-diff-del/5'
        : 'flex-1 min-w-0 self-stretch overflow-hidden flex flex-row items-start'}>
      {accentBar(line.kind)}
      {gutterCell(number, line.kind, true, step)}
      {markerCell(line.kind, true)}
      {codeViewport(line.text, wrapped, true)}
    </div>
  );
}

/// Rust: `split_filler`. The empty half of a one-sided split row. A flat
/// wash, quieter than either tint, reads as "nothing here".
export function splitFiller() {
  return <div className="flex-1 min-w-0 self-stretch bg-wash/5" />;
}

/// Rust: `split_row`. The two halves with the centre hairline between them.
export function splitRow(left: ReactNode, right: ReactNode, wrapped: boolean) {
  return (
    <div className={wrapped ? 'min-h-5 w-full flex-none flex flex-row items-stretch' : 'h-5 w-full flex-none flex flex-row items-stretch'}>
      {left}
      <div className="flex-none self-stretch border-l border-border" />
      {right}
    </div>
  );
}
