// Rust: app/crates/ui/src/transcript.rs (detail_body, more_lines_row,
// thought_line_text).
import type { ToolDetail } from './chip_header';
import { preserveSpaces } from './code_block';

/// Rust: `more_lines_row` - the counted tail under a truncated Output or
/// Thought detail.
function moreLinesRow(truncatedBy: number) {
  return (
    <div className="h-4 px-3 flex items-center text-ui-10 text-text-faint">
      {`… ${truncatedBy} more lines`}
    </div>
  );
}

/// Rust: `ToolDetail::Stats` - one row per file, the path truncating, then the
/// additions in success and the deletions in danger.
function statsBody(detail: Extract<ToolDetail, { type: 'stats' }>) {
  return (
    <div className="w-full min-w-0 flex flex-col overflow-hidden py-1.5 text-ui-11">
      {detail.stats.map(stat => (
        <div key={stat.path} className="h-4 w-full min-w-0 px-3 flex items-center gap-2">
          <div className="min-w-0 flex-1 truncate whitespace-nowrap text-text/88">{stat.path}</div>
          <div className="flex-none text-success">{`+${stat.additions}`}</div>
          <div className="flex-none text-danger">{`−${stat.deletions}`}</div>
        </div>
      ))}
    </div>
  );
}

/// Rust: `ToolDetail::Output` - fixed-height mono lines, each truncating.
function outputBody(detail: Extract<ToolDetail, { type: 'output' }>) {
  return (
    <div className="w-full min-w-0 flex flex-col overflow-hidden py-1.5 text-ui-11">
      {detail.lines.map((line, li) => (
        <div key={li} className="h-4 w-full min-w-0 px-3 flex items-center text-text/88">
          <div className="w-full min-w-0 truncate whitespace-nowrap">{preserveSpaces(line)}</div>
        </div>
      ))}
      {detail.truncatedBy > 0 && moreLinesRow(detail.truncatedBy)}
    </div>
  );
}

/// Rust: `ToolDetail::Thought` - the same fixed-height lines at 12px in the
/// sans face, since a thought is prose. A blank entry is a separator row.
function thoughtBody(detail: Extract<ToolDetail, { type: 'thought' }>) {
  return (
    <div className="w-full min-w-0 flex flex-col overflow-hidden py-1.5 text-ui-12">
      {detail.lines.map((line, li) => (
        <div key={li} className="h-4 w-full min-w-0 px-3 flex items-center text-text-muted">
          <div className="w-full min-w-0 truncate whitespace-nowrap">{preserveSpaces(line)}</div>
        </div>
      ))}
      {detail.truncatedBy > 0 && moreLinesRow(detail.truncatedBy)}
    </div>
  );
}

/// Rust: `detail_body`. The body under an open chip header, one hairline
/// below it: the invocation first (what was asked), then output or diff (what
/// came back).
export function detailBody(detail: ToolDetail) {
  switch (detail.type) {
    case 'stats':
      return statsBody(detail);
    case 'output':
      return outputBody(detail);
    case 'thought':
      return thoughtBody(detail);
  }
}

/// Rust: the 1px `hairline(0.06)` separator between a chip header and each
/// detail body it opens.
export function detailSeparator() {
  return <div className="h-0.25 flex-none bg-wash/5" />;
}
