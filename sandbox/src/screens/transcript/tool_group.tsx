// Rust: app/crates/ui/src/transcript.rs (render_tool_group, tool_chip,
// subagent_chip, tool_group_summary, chips_height).
import { chipHeader, chipHeaderRow } from './chip_header';
import type { ToolItem } from './chip_header';
import { detailBody, detailSeparator } from './detail';

/// Rust: the fold and detail-fold maps `Transcript` keeps per row id, plus the
/// two clicks that write them. `open(rowId)` answers the group accordion,
/// `detailOpen(key)` one chip's own body.
export type ToolGroupState = {
  open: (rowId: string, autoOpen: boolean) => boolean;
  detailOpen: (key: string, defaultOpen: boolean) => boolean;
  onToggleGroup: (rowId: string, autoOpen: boolean) => void;
  onToggleDetail: (key: string, defaultOpen: boolean) => void;
  onOpenSubagent: (docId: string, title: string) => void;
};

/// Rust: `is_spawn_link` - a chip that indexes a subagent doc is a link, not
/// an accordion, so it never expands.
function isSpawnLink(tool: ToolItem) {
  return tool.subagentRef !== undefined;
}

/// Rust: `tool_group_collapses` - a group of nothing but agent chips has no
/// header and never folds, so a running subagent stays visible.
function collapses(tools: readonly ToolItem[]) {
  return tools.some(tool => !isSpawnLink(tool));
}

/// Rust: the group guide rail - a 1px column the chips hang from, stretching
/// to the card so an open detail never breaks it.
function guideRail() {
  return <div className="ml-3 w-0.25 flex-none bg-wash/10" />;
}

/// Rust: the same rail with `.h_full()`, for the two chip rows that centre
/// their children - without it the rail collapses to nothing.
function guideRailFull() {
  return <div className="ml-3 h-full w-0.25 flex-none bg-wash/10" />;
}

/// Rust: `tool_chip` - a plain chip with no body to open.
function toolChip(tool: ToolItem, rail: boolean) {
  return (
    <div key={tool.id} data-chip={tool.id} className="h-9.5 w-full flex-none flex flex-row items-center">
      {rail && guideRailFull()}
      <div className={rail
        ? 'ml-3 h-7.5 min-w-0 flex-1 flex items-center overflow-hidden rounded-lg border border-wash/10 bg-wash/5'
        : 'h-7.5 min-w-0 flex-1 flex items-center overflow-hidden rounded-lg border border-wash/10 bg-wash/5'}>
        {chipHeaderRow(tool, undefined)}
      </div>
    </div>
  );
}

/// Rust: `subagent_chip` - the same card as `tool_chip`, but the WHOLE card is
/// the "open the subagent tab" click, with the open-arrow tile trailing.
function subagentChip(tool: ToolItem, rail: boolean, state: ToolGroupState) {
  const docId = tool.subagentRef ?? '';
  return (
    <div key={tool.id} data-chip={tool.id} className="h-9.5 w-full flex-none flex flex-row items-center">
      {rail && guideRailFull()}
      <button type="button" aria-label={`Open subagent: ${tool.detail}`}
        onClick={() => state.onOpenSubagent(docId, tool.detail)}
        className={rail
          ? 'ml-3 h-7.5 min-w-0 flex-1 flex items-center overflow-hidden rounded-lg border border-wash/10 bg-wash/5 cursor-pointer hover:bg-wash/10 active:bg-element-active focus:bg-wash/10'
          : 'h-7.5 min-w-0 flex-1 flex items-center overflow-hidden rounded-lg border border-wash/10 bg-wash/5 cursor-pointer hover:bg-wash/10 active:bg-element-active focus:bg-wash/10'}>
        {chipHeaderRow(tool, { type: 'openArrow' })}
      </button>
    </div>
  );
}

/// Rust: the expandable chip - ONE card whose header row is the chip and whose
/// body is the detail, never a floating card below it. Native gives the card an
/// explicit height so N chips cannot overflow the group by 2N px of border; the
/// browser's border-box does that on its own here.
function expandableChip(
  tool: ToolItem,
  rowId: string,
  ix: number,
  rail: boolean,
  state: ToolGroupState,
) {
  const key = `${rowId}#d${ix}`;
  // A streaming thought chip defaults open, a settled one closed. A user
  // toggle overrides either way.
  const defaultOpen = tool.isThought === true && tool.subagentStatus === 'running';
  const open = state.detailOpen(key, defaultOpen);
  return (
    <div key={tool.id} data-chip={tool.id} className="w-full flex-none flex flex-row">
      {rail && guideRail()}
      <div className="min-w-0 flex-1">
        <div className={rail
          ? 'my-1 ml-3 min-w-0 flex-1 flex flex-col overflow-hidden rounded-lg border border-wash/10 bg-wash/5'
          : 'my-1 min-w-0 flex-1 flex flex-col overflow-hidden rounded-lg border border-wash/10 bg-wash/5'}>
          <button type="button" data-chip-toggle={key} aria-expanded={open}
            onClick={() => state.onToggleDetail(key, defaultOpen)}
            className="h-7 flex-none flex items-center cursor-pointer text-left hover:bg-wash/5 active:bg-element-active focus:bg-wash/5">
            {chipHeader(tool, open)}
          </button>
          {open && tool.invocation !== undefined && detailSeparator()}
          {open && tool.invocation !== undefined && detailBody(tool.invocation)}
          {open && tool.body !== undefined && detailSeparator()}
          {open && tool.body !== undefined && detailBody(tool.body)}
        </div>
      </div>
    </div>
  );
}

/// Rust: `render_tool_group`. A small chevron tile centred over the chips'
/// guide rail, then the quiet 12px summary. The header stays muted even when
/// children failed: agents routinely have failed probes mid-work, and a red
/// header read as "this whole step broke". Failures show on the chips and in
/// the summary's failed count.
export function renderToolGroup(
  rowId: string,
  tools: readonly ToolItem[],
  autoOpen: boolean,
  summary: string,
  state: ToolGroupState,
) {
  const rail = collapses(tools);
  const open = !rail || state.open(rowId, autoOpen);
  const chips = (
    <div className="pt-0.5 flex flex-col">
      {tools.map((tool, ix) => {
        if (isSpawnLink(tool)) return subagentChip(tool, rail, state);
        if (tool.invocation === undefined && tool.body === undefined) return toolChip(tool, rail);
        return expandableChip(tool, rowId, ix, rail, state);
      })}
    </div>
  );
  return (
    <div className="flex flex-col">
      {rail && (
        <button type="button" data-group-toggle={rowId} aria-expanded={open}
          onClick={() => state.onToggleGroup(rowId, autoOpen)}
          className="flex flex-row items-center gap-2 px-1 h-6 cursor-pointer text-left text-ui-12 text-text-muted hover:text-text active:text-text focus:text-text">
          <div className="size-4 flex-none rounded-sm bg-wash/5 flex items-center justify-center text-ui-10 text-text-muted">
            {open ? '▾' : '▸'}
          </div>
          <div className="min-w-0 h-4 flex items-center truncate whitespace-nowrap">{summary}</div>
        </button>
      )}
      {open ? chips : <div className="overflow-hidden h-0" />}
    </div>
  );
}
