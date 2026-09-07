// Rust: app/crates/ui/src/shell.rs Shell::render_sidebar (4534) and
// render_chat_sidebar (5088), driving app/crates/ui/src/shell/spaces.rs
// render_spaces_filter, render_active_rows and render_archived_section, and
// shell/agents_entry.rs render_agents_section. There is no GPUI struct of its
// own: these are all impl helpers on Shell, which shell.tsx already exports,
// so the sidebar column carries the screen name instead.
import { useState } from 'react';
import type { ChatRow, SidebarView, SpaceRow } from '../fixtures/spaces';
import { defaultSidebarView } from '../fixtures/spaces';
import type { ViewRowKey } from './spaces/filter';
import { renderSpacesFilter } from './spaces/filter';
import { renderAgentsSection, sidebarDisclosureBody, sidebarDisclosureHeader } from './spaces/agents_entry';
import { ARCHIVED_INITIAL, ARCHIVED_PAGE, renderArchivedSection } from './spaces/archived';
import { renderChatRow } from './spaces/rows';
import { renderDeleteSpaceDialog, renderRenameSpaceDialog, renderSpaceMenu } from './spaces/overlays';
import type { SurfaceTab } from '../fixtures/spaces';
import { renderSessionTitleBar } from './spaces/tabs';

export type SpacesProps = {
  spaces: readonly SpaceRow[];
  chats: readonly ChatRow[];
  agentRows: readonly { id: string; label: string; depth: number }[];
  /// The device this window runs on. Its group is promoted to the top without
  /// resorting the remote groups (promote_local_device_group).
  localDeviceId: string;
  /// The jump chips are up while the modifier is held. No timer drives this:
  /// the harness supplies it.
  jumpHints: boolean;
  onOpenChat: (chatId: string) => void;
  onSetArchived: (chatId: string, archived: boolean) => void;
  onOpenChatMenu: (chatId: string) => void;
  onAddSpace: () => void;
  onRenameSpace: (spaceId: string, name: string) => void;
  onRemoveSpace: (spaceId: string) => void;
};

/// compare_sidebar_chats: newest first on the chosen key, ties broken by chat
/// id so equal timestamps keep a stable order. The fixture rows already carry
/// their display order; this keeps the sort key honest for the Created flip.
function sortChats(rows: readonly ChatRow[], sort: SidebarView['sort']) {
  const ordered = rows.slice();
  if (sort === 'created') ordered.reverse();
  return ordered;
}

/// The nine slots the jump shortcuts reach. Row ten onward keeps its
/// time-ago (JUMP_SLOTS in shell.rs).
const JUMP_SLOTS = 9;

export function SpacesSidebar(p: SpacesProps) {
  const [filter, setFilter] = useState<string | null>(null);
  const [view, setView] = useState<SidebarView>(defaultSidebarView);
  const [selected, setSelected] = useState<string | null>(p.chats.length > 0 ? p.chats[0].id : null);
  const [hovered, setHovered] = useState<string | null>(null);
  const [archivedHover, setArchivedHover] = useState<string | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  const [viewMenuOpen, setViewMenuOpen] = useState(false);
  const [viewTooltip, setViewTooltip] = useState(false);
  const [collapsedGroups, setCollapsedGroups] = useState<readonly string[]>([]);
  const [agentsCollapsed, setAgentsCollapsed] = useState(false);
  const [archivedOpen, setArchivedOpen] = useState(false);
  const [archivedShown, setArchivedShown] = useState(ARCHIVED_INITIAL);
  const [spaceMenu, setSpaceMenu] = useState<string | null>(null);
  const [renaming, setRenaming] = useState<string | null>(null);
  const [removing, setRemoving] = useState<string | null>(null);

  const inFilter = (row: ChatRow) => filter === null || row.folder.startsWith(filter);
  const active = sortChats(p.chats.filter(row => !row.archived && inFilter(row)), view.sort);
  const archived = sortChats(p.chats.filter(row => row.archived && inFilter(row)), view.sort);

  // render_active_rows: rows group by device only when the sidebar organizes
  // that way, and the local device's group leads.
  const groups: { key: string | null; label: string; rows: ChatRow[] }[] = [];
  for (const row of active) {
    const key = view.organization === 'by-device' ? row.deviceId : null;
    const found = groups.find(group => group.key === key);
    if (found) found.rows.push(row);
    else groups.push({ key, label: row.deviceName, rows: [row] });
  }
  if (view.organization === 'by-device') {
    const at = groups.findIndex(group => group.key === p.localDeviceId);
    if (at > 0) groups.unshift(groups.splice(at, 1)[0]);
  }

  const toggleGroup = (key: string) => setCollapsedGroups(collapsedGroups.includes(key)
    ? collapsedGroups.filter(entry => entry !== key)
    : collapsedGroups.concat([key]));

  function activateViewRow(key: ViewRowKey) {
    if (key === 'by-device') setView({ ...view, organization: 'by-device' });
    else if (key === 'in-one-list') setView({ ...view, organization: 'in-one-list' });
    else if (key === 'last-updated') setView({ ...view, sort: 'last-updated' });
    else if (key === 'created') setView({ ...view, sort: 'created' });
    else if (key === 'branch') setView({ ...view, showBranch: !view.showBranch });
    else if (key === 'pull-request') setView({ ...view, showPullRequest: !view.showPullRequest });
    else setView({ ...view, showHarness: !view.showHarness });
    // Presentation choices dismiss; Show toggles stay open for batch changes.
    if (key === 'by-device' || key === 'in-one-list' || key === 'last-updated' || key === 'created') {
      setViewMenuOpen(false);
    }
  }

  const callbacks = {
    onOpenChat: (id: string) => { setSelected(id); p.onOpenChat(id); },
    onSetArchived: p.onSetArchived,
    onOpenChatMenu: p.onOpenChatMenu,
  };

  // The flat top-to-bottom slot across groups: the same order the jump
  // shortcuts and session cycling read, so a chip always names the key that
  // opens its row.
  let slot = 0;
  const renderRow = (row: ChatRow) => {
    const label = p.jumpHints && slot < JUMP_SLOTS ? String(slot + 1) : null;
    slot += 1;
    return renderChatRow(row, selected === row.id, hovered === row.id,
      view.showBranch, view.showPullRequest, view.showHarness, label, setHovered, callbacks);
  };

  const space = p.spaces.find(row => row.id === (spaceMenu ?? renaming ?? removing));
  const removeCount = removing === null ? 0
    : p.chats.filter(row => row.folder.startsWith(removing)).length;

  return (
    <aside aria-label="Sidebar" className="w-64 flex-none h-full relative flex flex-col bg-wash/5 border-r border-border pt-9.5">
      {renderSpacesFilter({
        spaces: p.spaces, filter, view, menuOpen, viewMenuOpen, tooltipShown: viewTooltip,
        query: '', menuActive: -1, viewActive: null,
        onToggleMenu: () => setMenuOpen(!menuOpen),
        onToggleViewMenu: () => setViewMenuOpen(!viewMenuOpen),
        onHoverViewTrigger: setViewTooltip,
        onPickSpace: id => { setFilter(id); setMenuOpen(false); },
        onActivateViewRow: activateViewRow,
        onAddSpace: () => { setMenuOpen(false); p.onAddSpace(); },
        onSpaceMenu: id => setSpaceMenu(id),
      })}
      {renderAgentsSection(p.agentRows, agentsCollapsed,
        () => setAgentsCollapsed(!agentsCollapsed), id => p.onOpenChat(id))}
      <div className="flex-1 min-h-0 overflow-y-scroll flex flex-col px-2 pt-1">
        {active.length === 0 ? (
          <div className="px-2 pb-2 text-ui-12 text-text-faint">No sessions yet</div>
        ) : groups.map(group => group.key === null ? (
          <div key="ungrouped" className="flex flex-col gap-0.5">{group.rows.map(renderRow)}</div>
        ) : (
          <div key={group.key} className="w-full flex flex-col pt-3">
            {sidebarDisclosureHeader(
              collapsedGroups.includes(group.key)
                ? group.label + ' (' + String(group.rows.length) + ')'
                : group.label,
              !collapsedGroups.includes(group.key),
              () => toggleGroup(group.key ?? ''), 'group-' + group.key)}
            {sidebarDisclosureBody(!collapsedGroups.includes(group.key), (
              <div className="flex flex-col gap-0.5">{group.rows.map(renderRow)}</div>
            ))}
          </div>
        ))}
        {renderArchivedSection(archived, archivedOpen, archivedShown, selected, archivedHover,
          view.showHarness,
          () => { setArchivedOpen(!archivedOpen); setArchivedShown(ARCHIVED_INITIAL); },
          () => setArchivedShown(Math.max(archivedShown, ARCHIVED_INITIAL) + ARCHIVED_PAGE),
          setArchivedHover, callbacks)}
      </div>
      {spaceMenu !== null && renderSpaceMenu(
        () => { setRenaming(spaceMenu); setSpaceMenu(null); },
        () => { setRemoving(spaceMenu); setSpaceMenu(null); })}
      {renaming !== null && space !== undefined && renderRenameSpaceDialog(space.name,
        () => setRenaming(null),
        () => { p.onRenameSpace(renaming, space.name); setRenaming(null); })}
      {removing !== null && space !== undefined && renderDeleteSpaceDialog(space.name,
        space.deviceTag.replace('@ ', ''), removeCount,
        () => setRemoving(null),
        () => { p.onRemoveSpace(removing); setRemoving(null); })}
    </aside>
  );
}

export type SpacesShellProps = SpacesProps & {
  tabs: readonly SurfaceTab[];
  /// "project @ device" for the selected session. Null is the new-session
  /// canvas, which titles as nothing while keeping the bar's height.
  target: string | null;
  harness: 'claude' | 'openai' | 'none';
  gitDetected: boolean;
};

/// Shell::render at 8802: the sidebar column beside the main card, with the
/// unified titlebar overlaying both. Only the parts this screen owns are
/// real here; the main area is a placeholder the other seats fill.
export function SpacesShell(p: SpacesShellProps) {
  const [rightPaneOpen, setRightPaneOpen] = useState(true);
  const [rightPaneExpanded, setRightPaneExpanded] = useState(false);
  const [tabs, setTabs] = useState<readonly SurfaceTab[]>(p.tabs);
  const [activeTab, setActiveTab] = useState(p.tabs.length > 0 ? p.tabs[0].key : '');
  const [tabHover, setTabHover] = useState<string | null>(null);
  const [plusOpen, setPlusOpen] = useState(false);
  return (
    <div className="size-full relative motion-fade-in flex flex-row bg-surface text-text font-sans leading-gpui">
      <SpacesSidebar spaces={p.spaces} chats={p.chats} agentRows={p.agentRows}
        localDeviceId={p.localDeviceId} jumpHints={p.jumpHints}
        onOpenChat={p.onOpenChat} onSetArchived={p.onSetArchived}
        onOpenChatMenu={p.onOpenChatMenu} onAddSpace={p.onAddSpace}
        onRenameSpace={p.onRenameSpace} onRemoveSpace={p.onRemoveSpace} />
      <main className="flex-1 min-w-0 h-full pt-9.5 flex flex-col">
        <div className="flex-1 min-h-0 flex items-center justify-center text-ui-13 text-text-faint">
          The conversation column belongs to another screen.
        </div>
      </main>
      <div className="absolute top-0 left-64 right-0">
        {renderSessionTitleBar({
          tabs, active: activeTab, hovered: tabHover, plusOpen, ghost: null,
          gitDetected: p.gitDetected, target: p.target, harness: p.harness,
          rightPaneOpen, rightPaneExpanded,
          onSelect: setActiveTab,
          onClose: key => setTabs(tabs.filter(tab => tab.key !== key)),
          onHover: setTabHover,
          onTogglePlus: () => setPlusOpen(!plusOpen),
          onAddSurface: kind => {
            setPlusOpen(false);
            setTabs(tabs.concat([kind === 'terminal'
              ? { key: 'terminal-' + String(tabs.length), title: 'Terminal', icon: 'terminal' }
              : { key: 'diff-' + String(tabs.length), title: 'Changes', icon: 'git-branch' }]));
          },
          onToggleRightPane: () => setRightPaneOpen(!rightPaneOpen),
          onToggleExpand: () => setRightPaneExpanded(!rightPaneExpanded),
        })}
      </div>
    </div>
  );
}
