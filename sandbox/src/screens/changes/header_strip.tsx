// Rust: app/crates/ui/src/changes.rs, Changes::render_header_strip.
import type { DiffScope, ParsedDiff } from './model';
import { scopeLabel } from './model';

/// The strip above the list: what the pane is diffing, the two totals, and
/// the partial-snapshot badge when the patch was capped.
export function renderHeaderStrip(parsed: ParsedDiff, scope: DiffScope, base: string | null) {
  return (
    <div className="flex-none h-9 flex flex-row items-center gap-2.5 px-4 border-b border-border">
      <div className="min-w-0 truncate text-ui-12 text-text-muted">{scopeLabel(scope, parsed.files.length, base)}</div>
      <div className="text-ui-11 text-diff-add">{`+${parsed.additions}`}</div>
      <div className="text-ui-11 text-diff-del">{`−${parsed.deletions}`}</div>
      <div className="flex-1" />
      {parsed.truncated && (
        <div className="flex-none text-ui-10 px-1.5 py-0.25 rounded-sm bg-warning/10 text-warning/88">Partial snapshot</div>
      )}
    </div>
  );
}
