// Rust: app/crates/ui/src/transcript.rs (chip_header_row, chip_header,
// ChipTrail, tool_icon_path).
import { icon } from './icons';
import type { IconName } from './icons';
import { miniGlyphSpinner } from './loaders';

/// Rust: `ToolItem` in transcript.rs, reduced to the fields the chip paints.
/// `label`/`detail` come from `surya_proto::view::tool_chip_content`, so the
/// fixture carries them already split rather than re-deriving the call here.
export type ToolItem = {
  id: string;
  glyph: IconName;
  label: string;
  detail: string;
  isThought?: boolean;
  isError?: boolean;
  model?: string;
  subagentRef?: string;
  subagentStatus?: 'running' | 'done' | 'failed';
  invocation?: ToolDetail;
  body?: ToolDetail;
};

/// Rust: `ToolDetail`. `Diff` carries the changes pane's own file body, which
/// is not ported here; `Stats`, `Output` and `Thought` are.
export type ToolDetail =
  | { type: 'output'; lines: readonly string[]; truncatedBy: number }
  | { type: 'thought'; lines: readonly string[]; truncatedBy: number }
  | { type: 'stats'; stats: readonly { path: string; additions: number; deletions: number }[] };

/// Rust: `ChipTrail` - the expand chevron, or the spawn chip's open arrow.
export type ChipTrail = { type: 'chevron'; open: boolean } | { type: 'openArrow' };

/// Rust: `chip_header_row`. Icon tile, medium label, truncating detail, then
/// the optional model text, the running spinner and the trailing tile.
/// A failed chip tints the label and the detail danger; the header of the
/// group it lives in stays quiet on purpose.
export function chipHeaderRow(tool: ToolItem, trail: ChipTrail | undefined) {
  const running = tool.subagentRef !== undefined && tool.subagentStatus === 'running';
  const failed = tool.isError === true
    || (tool.subagentRef !== undefined && tool.subagentStatus === 'failed');
  const label = tool.isThought === true ? 'Thought process' : tool.label;
  const detail = tool.isThought === true ? '' : tool.detail;
  return (
    <div className="h-7 w-full min-w-0 flex flex-row items-center gap-2 px-2 text-ui-12">
      <div className="size-4 flex-none rounded-sm bg-wash/10 flex items-center justify-center text-text-muted">
        <span className="size-3 flex items-center justify-center">
          {icon(tool.isThought === true ? 'chat-round-line' : tool.glyph)}
        </span>
      </div>
      <div className={failed
        ? 'flex-none h-4 flex items-center font-medium text-danger'
        : 'flex-none h-4 flex items-center font-medium text-text-muted'}>{label}</div>
      <div className={failed
        ? 'flex-1 min-w-0 h-4 flex items-center truncate whitespace-nowrap text-danger'
        : 'flex-1 min-w-0 h-4 flex items-center truncate whitespace-nowrap text-text/88'}>{detail}</div>
      {tool.model !== undefined && (
        <div className="flex-none h-4 flex items-center text-ui-11 text-text-faint">{tool.model}</div>
      )}
      {running && <div className="flex-none">{miniGlyphSpinner()}</div>}
      {trail !== undefined && trailTile(trail)}
    </div>
  );
}

/// Rust: the trailing tile of `chip_header_row` - it matches the group
/// header's tile, and carries either the accordion chevron or the open arrow.
function trailTile(trail: ChipTrail) {
  if (trail.type === 'openArrow') {
    return (
      <div className="size-4 flex-none rounded-sm bg-wash/5 flex items-center justify-center text-text-muted">
        <span className="size-2.5 flex items-center justify-center">{icon('arrow-up-right')}</span>
      </div>
    );
  }
  return (
    <div className="size-4 flex-none rounded-sm bg-wash/5 flex items-center justify-center text-ui-10 text-text-muted">
      {trail.open ? '▾' : '▸'}
    </div>
  );
}

/// Rust: `chip_header` - the header row of an expandable chip card.
export function chipHeader(tool: ToolItem, open: boolean) {
  return chipHeaderRow(tool, { type: 'chevron', open });
}
