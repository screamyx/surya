// Rust: app/crates/ui/src/shell/spaces.rs - render_spaces_filter (727),
// render_spaces_menu (936), render_sidebar_view_menu (620) and
// SidebarViewOptionsTooltip (67). Render helpers on the same pane.
import type { SidebarView, SpaceRow } from '../../fixtures/spaces';
import { menuHeading, menuRow, menuSeparator, paintedQuery, popoverCard, searchInputFrame } from './chrome';
import { renderIcon } from './icons';

/// SIDEBAR_VIEW_ROWS, with the labels, icons and settings field each reads.
/// The first four are radio-style and dismiss the menu; the Show toggles stay
/// open for batch changes (SidebarViewRow::closes_menu).
const VIEW_ROWS = [
  { key: 'by-device', label: 'By device', icon: 'laptop', group: 'organize' },
  { key: 'in-one-list', label: 'In one list', icon: 'list', group: 'organize' },
  { key: 'last-updated', label: 'Last updated', icon: 'clock-circle', group: 'sort' },
  { key: 'created', label: 'Created', icon: 'calendar', group: 'sort' },
  { key: 'branch', label: 'Branch', icon: 'git-branch', group: 'show' },
  { key: 'pull-request', label: 'Pull request', icon: 'pull-request', group: 'show' },
  { key: 'harness', label: 'Harness', icon: 'bot', group: 'show' },
] as const;

export type ViewRowKey = (typeof VIEW_ROWS)[number]['key'];

function viewRowSelected(key: ViewRowKey, view: SidebarView) {
  if (key === 'by-device') return view.organization === 'by-device';
  if (key === 'in-one-list') return view.organization === 'in-one-list';
  if (key === 'last-updated') return view.sort === 'last-updated';
  if (key === 'created') return view.sort === 'created';
  if (key === 'branch') return view.showBranch;
  if (key === 'pull-request') return view.showPullRequest;
  return view.showHarness;
}

/// SidebarViewOptionsTooltip: an 11px label on a raised card, shown after the
/// native 350ms delay. There is no admitted delay, so it follows hover.
export function sidebarViewOptionsTooltip() {
  return <span role="tooltip" className="absolute top-7.5 right-0 px-2 py-1.5 rounded-md border border-border-strong bg-surface-raised text-ui-11 text-text whitespace-nowrap">Sidebar view options</span>;
}

/// render_sidebar_view_menu: three labelled sections over one row list, each
/// row a mark, a label, and a check on the selected ones.
export function renderSidebarViewMenu(view: SidebarView, active: number | null,
  onActivate: (key: ViewRowKey) => void) {
  const section = (group: string) => VIEW_ROWS.map((row, ix) => ({ row, ix }))
    .filter(entry => entry.row.group === group)
    .map(({ row, ix }) => menuRow(false, active === ix, () => onActivate(row.key),
      'sidebar-view-row-' + String(ix), (
        <>
          <span className="size-3.75 flex-none text-text-muted/88">{renderIcon(row.icon)}</span>
          <span className="flex-1 min-w-0 truncate whitespace-nowrap">{row.label}</span>
          <span className="w-3.5 flex-none">
            {viewRowSelected(row.key, view) && <span className="size-3.5 text-text-muted">{renderIcon('check')}</span>}
          </span>
        </>
      )));
  return (
    <div className="absolute top-7.5 right-0 w-55">
      <div className="relative motion-menu-in">
        {popoverCard(
          <div className="flex flex-col">
            {menuHeading('Organize')}
            <div className="flex flex-col gap-0.5">{section('organize')}</div>
            {menuSeparator('sep-sort')}
            {menuHeading('Sort')}
            <div className="flex flex-col gap-0.5">{section('sort')}</div>
            {menuSeparator('sep-show')}
            {menuHeading('Show')}
            <div className="flex flex-col gap-0.5">{section('show')}</div>
          </div>
        )}
      </div>
    </div>
  );
}

/// render_spaces_menu: search on top, then "All projects", the space rows
/// (each with its "@ device" tag and an offline glyph), and "New project...".
/// The selected row's wash is the only selection signal; there is no check.
export function renderSpacesMenu(spaces: readonly SpaceRow[], filter: string | null,
  query: string, active: number, onPick: (id: string | null) => void,
  onAddSpace: () => void, onSpaceMenu: (id: string) => void) {
  const matches = spaces.filter(space => space.name.toLowerCase().includes(query.toLowerCase()));
  return (
    <div className="absolute top-7.5 left-0 w-55">
      <div className="relative motion-menu-in">
        {popoverCard(
          <div className="flex flex-col">
            {searchInputFrame(paintedQuery(query, 'Search projects'))}
            <div className="flex flex-col gap-0.5 max-h-80 overflow-y-scroll">
              {menuRow(filter === null, active === 0, () => onPick(null), 'spaces-menu-row-all', (
                <>
                  <span className="size-3.75 flex-none text-text-muted/88">{renderIcon('folder')}</span>
                  <span className="flex-1 min-w-0 truncate whitespace-nowrap">All projects</span>
                </>
              ))}
              {matches.map((space, ix) => (
                <div key={space.id} onContextMenu={() => onSpaceMenu(space.id)}>
                  {menuRow(filter === space.id, active === ix + 1, () => onPick(space.id),
                    'spaces-menu-row-' + space.id, (
                      <>
                        <span className="size-3.75 flex-none text-text-muted/88">{renderIcon('folder')}</span>
                        <span className="flex-1 min-w-0 truncate whitespace-nowrap">{space.name}</span>
                        <span className="flex-none text-ui-10 text-text-muted/50">{space.deviceTag}</span>
                        {space.offline && <span className="size-3 flex-none text-warning/88">{renderIcon('wifi-off')}</span>}
                      </>
                    ))}
                </div>
              ))}
              {menuRow(false, active === matches.length + 1, onAddSpace, 'spaces-menu-row-add', (
                <>
                  <span className="size-3.75 flex-none text-text-muted/88">{renderIcon('plus')}</span>
                  <span className="flex-1 min-w-0 truncate whitespace-nowrap">New project...</span>
                </>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export type FilterProps = {
  spaces: readonly SpaceRow[];
  filter: string | null;
  view: SidebarView;
  menuOpen: boolean;
  viewMenuOpen: boolean;
  tooltipShown: boolean;
  query: string;
  menuActive: number;
  viewActive: number | null;
  onToggleMenu: () => void;
  onToggleViewMenu: () => void;
  onHoverViewTrigger: (hovered: boolean) => void;
  onPickSpace: (id: string | null) => void;
  onActivateViewRow: (key: ViewRowKey) => void;
  onAddSpace: () => void;
  onSpaceMenu: (id: string) => void;
};

/// render_spaces_filter: the current filter with its chevron, the view-options
/// square beside it, and both dropdowns floating beneath. Sits OUTSIDE the
/// list's scroll region so neither float can be clipped.
export function renderSpacesFilter(p: FilterProps) {
  const space = p.spaces.find(row => row.id === p.filter);
  return (
    <div className="flex-none flex flex-row items-center gap-1 px-2 pt-2 pb-1">
      <div className="flex-1 min-w-0 relative">
        <button type="button" onClick={p.onToggleMenu} data-trigger="spaces-filter"
          aria-expanded={p.menuOpen} aria-haspopup="menu"
          className={p.menuOpen
            ? 'w-full h-7.5 flex flex-row items-center gap-2 rounded-lg px-2 text-ui-13 font-medium text-text bg-element-hover cursor-pointer'
            : 'w-full h-7.5 flex flex-row items-center gap-2 rounded-lg px-2 text-ui-13 font-medium text-text/88 cursor-pointer motion-hover-fade hover:bg-element-hover hover:text-text active:bg-element-active focus:bg-element-hover'}>
          <span className="size-4 flex-none text-text-muted">{renderIcon('folder')}</span>
          <span className="flex-1 min-w-0 flex flex-row items-center gap-1.5">
            <span className="min-w-0 truncate whitespace-nowrap">{space ? space.name : 'All projects'}</span>
            {space && <span className="flex-none text-ui-10 font-normal text-text-muted/50">{space.deviceTag}</span>}
            {space !== undefined && space.offline && <span className="size-3 flex-none text-warning/88">{renderIcon('wifi-off')}</span>}
          </span>
          <span className="size-3.5 flex-none text-text-muted/50">{renderIcon('alt-arrow-down')}</span>
        </button>
        {p.menuOpen && renderSpacesMenu(p.spaces, p.filter, p.query, p.menuActive,
          p.onPickSpace, p.onAddSpace, p.onSpaceMenu)}
      </div>
      <div className="flex-none relative">
        <button type="button" onClick={p.onToggleViewMenu} data-trigger="sidebar-view-options"
          aria-label="Sidebar view options" aria-expanded={p.viewMenuOpen} aria-haspopup="menu"
          onMouseEnter={() => p.onHoverViewTrigger(true)} onMouseLeave={() => p.onHoverViewTrigger(false)}
          className={p.viewMenuOpen
            ? 'size-7.5 flex-none flex items-center justify-center rounded-lg border border-border-strong text-text-muted bg-element-hover cursor-pointer'
            : 'size-7.5 flex-none flex items-center justify-center rounded-lg border border-surface text-text-muted cursor-pointer hover:bg-element-hover active:bg-element-active focus:border-border-strong focus:bg-element-hover'}>
          <span className="size-4">{renderIcon('sort')}</span>
        </button>
        {p.tooltipShown && !p.viewMenuOpen && sidebarViewOptionsTooltip()}
        {p.viewMenuOpen && renderSidebarViewMenu(p.view, p.viewActive, p.onActivateViewRow)}
      </div>
    </div>
  );
}
