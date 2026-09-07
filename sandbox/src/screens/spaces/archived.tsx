// Rust: app/crates/ui/src/shell/spaces.rs render_archived_section (1331).
// The settled shelf below the active list: a disclosure header carrying the
// count while closed, 36px one-liner rows, and a "Show N more" pager.
import type { ChatRow } from '../../fixtures/spaces';
import type { ChatRowCallbacks } from './rows';
import { renderArchivedRow } from './rows';
import { sidebarDisclosureBody, sidebarDisclosureHeader } from './agents_entry';
import { renderIcon } from '../../icons';

/// The native pager: ten rows to start, twenty-five more a click.
export const ARCHIVED_INITIAL = 10;
export const ARCHIVED_PAGE = 25;

export function renderArchivedSection(rows: readonly ChatRow[], open: boolean, shown: number,
  selected: string | null, hovered: string | null, showHarness: boolean,
  onToggle: () => void, onShowMore: () => void,
  onHover: (id: string | null) => void, cb: ChatRowCallbacks) {
  // None when nothing is archived under the current project filter.
  if (rows.length === 0) return null;
  const total = rows.length;
  const limit = Math.max(shown, ARCHIVED_INITIAL);
  const visible = rows.slice(0, limit);
  const hasMore = total > limit;
  const remaining = Math.min(total - limit, ARCHIVED_PAGE);
  // The count only shows while collapsed; expanded, the rows speak for
  // themselves.
  const label = open ? 'Archived' : 'Archived (' + String(total) + ')';
  return (
    <div className="flex flex-col pt-3">
      {sidebarDisclosureHeader(label, open, onToggle, 'archived')}
      {sidebarDisclosureBody(open, (
        <>
          <div className="flex flex-col gap-0.5">
            {visible.map(row => renderArchivedRow(row, selected === row.id,
              hovered === row.id, showHarness, onHover, cb))}
          </div>
          {hasMore && (
            <button type="button" onClick={onShowMore} data-archived-more="1"
              className="mt-0.5 h-9 flex flex-row items-center gap-2.5 px-2 rounded-md text-ui-13 text-text-muted/50 text-left cursor-pointer hover:bg-element-hover hover:text-text active:bg-element-active focus:bg-element-hover">
              <span className="size-3.5 flex-none">{renderIcon('plus')}</span>
              <span>Show {remaining} more</span>
            </button>
          )}
        </>
      ))}
    </div>
  );
}
