// Rust: app/crates/ui/src/pickers.rs - render_branch_popover (2794), the ref
// picker: search on top, rows with right-aligned muted current/worktree tags,
// and a "Showing X of Y refs" footer when the list is capped at MAX_REF_ROWS.
import { menuRowNav, menuSection, popoverNote, searchBox, skeletonRows, errorRow } from './popover';
import type { RepoRef } from '../../fixtures/pickers';

export type BranchPopoverProps = {
  rows: readonly RepoRef[]; total: number; hasSpace: boolean; query: string;
  selected: string | null; active: number; switching: string | null;
  switchError: string | null; loading: boolean; error: string | null;
  onPick: (name: string) => void; onHover: (index: number) => void; onRetry: () => void;
};

export function renderBranchPopover(props: BranchPopoverProps) {
  if (!props.hasSpace) return popoverNote('No project selected');
  const shown = props.rows.length;
  const body = props.loading
    ? skeletonRows(4)
    : props.error
      ? errorRow(props.error, props.onRetry)
      : shown === 0
        ? popoverNote('No refs found.')
        : (
          <div className={props.switching
            ? 'flex flex-col gap-0.5 max-h-55 overflow-y-scroll opacity-55'
            : 'flex flex-col gap-0.5 max-h-55 overflow-y-scroll'}>
            {props.rows.map((row, ix) => menuRowNav(
              `branch-row-${ix}`, props.selected === row.name, ix === props.active,
              () => props.onPick(row.name), () => props.onHover(ix),
              <>
                <span className="flex-1 min-w-0 truncate">{row.name}</span>
                {props.switching === row.name && (
                  <span className="flex-none text-ui-10 text-text-faint">switching…</span>
                )}
                {row.current
                  ? <span className="flex-none text-ui-10 text-text-faint">current</span>
                  : row.worktree
                    ? <span className="flex-none text-ui-10 text-text-faint">worktree</span>
                    : null}
              </>,
            ))}
          </div>
        );
  return (
    <div className="flex flex-col">
      {searchBox(props.query, 'Search refs…')}
      {body}
      {props.switchError && menuSection(
        <div className="px-2 py-1 text-ui-11 text-danger">{props.switchError}</div>,
      )}
      {props.total > shown && menuSection(
        <div className="px-2 py-1 text-ui-11 text-text-faint">{`Showing ${shown} of ${props.total} refs`}</div>,
      )}
    </div>
  );
}
