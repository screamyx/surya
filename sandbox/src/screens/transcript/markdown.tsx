// Rust: app/crates/ui/src/markdown/render.rs (render_block, flatten_runs,
// heading_metrics, flat_text_element's inline-code wash).
import { renderCodeBlock } from './code_block';
import type { CodeState } from './code_block';

/// Rust: `surya_doc` InlineRun + InlineStyle, flattened by `flatten_runs`.
export type InlineRun = {
  text: string;
  bold?: boolean;
  italic?: boolean;
  code?: boolean;
  link?: boolean;
};

/// Rust: `crate::markdown::parser::Block`.
export type MdBlock =
  | { type: 'paragraph'; runs: readonly InlineRun[] }
  | { type: 'heading'; level: number; runs: readonly InlineRun[] }
  | { type: 'code'; language?: string; code: string }
  | { type: 'quote'; children: readonly MdBlock[] }
  | { type: 'list'; orderedStart?: number; items: readonly (readonly MdBlock[])[] }
  | { type: 'rule' };

/// Rust: `flatten_runs` plus the inline-code underlay `flat_text_element`
/// paints. Inline code shapes in the mono font at `inline_code_text` over the
/// `inline_code_wash` rounded quad; every other run inherits the block's tone.
function runSpan(run: InlineRun, key: number) {
  if (run.code) {
    return <span key={key} className="rounded-sm px-0.5 bg-code-wash text-code-text">{run.text}</span>;
  }
  if (run.link) {
    return <span key={key} className="text-accent">{run.text}</span>;
  }
  if (run.bold) {
    return <span key={key} className="font-semibold text-text">{run.text}</span>;
  }
  if (run.italic) {
    return <span key={key} className="italic">{run.text}</span>;
  }
  return <span key={key}>{run.text}</span>;
}

export function inlineRuns(runs: readonly InlineRun[]) {
  return runs.map((run, ix) => runSpan(run, ix));
}

/// Rust: `heading_metrics` - h1 19/27, h2 16/24, h3 15/22, else 14/22, all
/// semibold on `text`. `text-ui-*` carries the gpui leading, so no line class.
function heading(level: number, runs: readonly InlineRun[]) {
  if (level <= 1) {
    return <h1 className="text-ui-20 font-semibold text-text">{inlineRuns(runs)}</h1>;
  }
  if (level === 2) {
    return <h2 className="text-ui-16 font-semibold text-text">{inlineRuns(runs)}</h2>;
  }
  if (level === 3) {
    return <h3 className="text-ui-14 font-semibold text-text">{inlineRuns(runs)}</h3>;
  }
  return <h4 className="text-ui-14 font-semibold text-text">{inlineRuns(runs)}</h4>;
}

/// Rust: `Block::List`. Ordered markers are accent-tinted numbers in an 18px
/// gutter; unordered is a real 5px accent disc centred on the first line's cap
/// band, because the bullet glyph reads too small at 14px.
function listItem(
  orderedStart: number | undefined,
  itemIx: number,
  item: readonly MdBlock[],
  rowKey: string,
  code: CodeState,
) {
  const marker = orderedStart === undefined ? (
    <div className="flex-none min-w-4 h-5 flex items-center">
      <div className="ml-0.25 size-1 rounded-full bg-accent" />
    </div>
  ) : (
    <div className="flex-none min-w-4 text-ui-14 text-accent">{`${orderedStart + itemIx}.`}</div>
  );
  return (
    <div key={itemIx} className="flex flex-row gap-2">
      {marker}
      <div className="flex-1 min-w-0 flex flex-col gap-1">
        {item.map((child, ci) => (
          <div key={ci}>{renderBlock(child, `${rowKey}-l${itemIx}-${ci}`, code)}</div>
        ))}
      </div>
    </div>
  );
}

/// Rust: `render_block`. One top-level block per transcript row; nested blocks
/// recurse through the same function, exactly as the native one does.
export function renderBlock(block: MdBlock, rowKey: string, code: CodeState) {
  switch (block.type) {
    case 'paragraph':
      return <p className="text-ui-14 text-text">{inlineRuns(block.runs)}</p>;
    case 'heading':
      return heading(block.level, block.runs);
    case 'code':
      return renderCodeBlock(block.language, block.code, rowKey, code);
    case 'quote':
      return (
        <div className="border-l border-accent/50 bg-accent/5 rounded-md pl-3 pr-2.5 py-1.5 flex flex-col gap-2 text-ui-14 text-text-muted">
          {block.children.map((child, ci) => (
            <div key={ci}>{renderBlock(child, `${rowKey}-q${ci}`, code)}</div>
          ))}
        </div>
      );
    case 'list':
      return (
        <div className="flex flex-col gap-1">
          {block.items.map((item, ix) => listItem(block.orderedStart, ix, item, rowKey, code))}
        </div>
      );
    case 'rule':
      return <div className="h-0.25 w-full bg-border" />;
  }
}
