// Rust: app/crates/ui/src/shell/agents_entry.rs (render_agents_section,
// agents_tree_shown, collapsed_after_rail_click) plus the two disclosure
// helpers it shares with the groups and the archived shelf,
// shell/spaces.rs sidebar_disclosure_header (155) and
// sidebar_disclosure_chevron (333).
import type { ReactNode } from 'react';
import { renderIcon } from '../../icons';

/// Is the agent tree on screen? Both what the section renders on and what the
/// rail entry lights on, so the row can never disagree with what is below it.
export function agentsTreeShown(railBuilt: boolean, collapsed: boolean) {
  return railBuilt && !collapsed;
}

/// Where a click on the rail's Agents entry leaves the section. The input is
/// deliberately ignored: a rail entry reveals what it names, never hides it.
export function collapsedAfterRailClick(_wasCollapsed: boolean) {
  return false;
}

/// sidebar_disclosure_chevron. The native chevron is ALT_ARROW_RIGHT rotated
/// a quarter turn when open, tweened by COLLAPSE. Rotation has no admitted
/// pair, so the open state swaps to the down mark and the turn is a gap.
export function sidebarDisclosureChevron(open: boolean) {
  return <span className="size-3 flex-none text-text-muted/50" data-chevron={open ? 'open' : 'closed'}>
    {renderIcon(open ? 'alt-arrow-down' : 'alt-arrow-right')}
  </span>;
}

/// sidebar_disclosure_header: 28px tall, a 12px medium label at half-muted, a
/// hairline filling the middle, and the chevron. The whole row is the toggle.
export function sidebarDisclosureHeader(label: string, open: boolean,
  onToggle: () => void, key: string) {
  return (
    <button key={key} type="button" onClick={onToggle} data-disclosure={key}
      aria-expanded={open}
      className="w-full flex flex-row items-center gap-2 h-7 px-2 cursor-pointer text-left hover:bg-element-hover active:bg-element-active focus:bg-element-hover">
      <span className="flex-none text-ui-12 font-medium text-text-muted/50">{label}</span>
      <span className="h-0.25 flex-1 bg-border" />
      {sidebarDisclosureChevron(open)}
    </button>
  );
}

/// render_sidebar_disclosure_body. The native body tweens its height with
/// COLLAPSE and rides a 0.35-to-1 opacity reveal; here it is present or gone.
/// motion-collapse transitions height, and spaces.rs:1265 computes the target
/// in pixels from the row count. The sandbox has no admitted way to set an
/// arbitrary pixel height, so the tween stays a gap rather than a guess.
export function sidebarDisclosureBody(open: boolean, children: ReactNode) {
  if (!open) return null;
  return <div className="w-full flex-none flex flex-col pt-1 overflow-hidden">{children}</div>;
}

/// render_agents_section: the collapsible header, and the tree under it while
/// expanded. The header renders whether or not the tree does, because it is
/// the only way back once the section is collapsed. The tree itself is
/// inbox/agents.rs and belongs to the agents-rail screen; the rows here are
/// fixture stand-ins so the frame reads at the right height.
export function renderAgentsSection(rows: readonly { id: string; label: string; depth: number }[],
  collapsed: boolean, onToggle: () => void, onOpenAgent: (id: string) => void) {
  if (rows.length === 0) return null;
  return (
    <div className="flex-none border-b border-border flex flex-col">
      {sidebarDisclosureHeader('Agents', !collapsed, onToggle, 'agents-tree')}
      {sidebarDisclosureBody(!collapsed, (
        <div className="flex-none max-h-55 overflow-y-scroll flex flex-col px-2 pb-2">
          {rows.map(row => (
            <button key={row.id} type="button" onClick={() => onOpenAgent(row.id)} data-agent={row.id}
              className={row.depth === 0
                ? 'flex flex-row items-center gap-2 h-7 px-2 rounded-md text-ui-13 text-text/88 text-left cursor-pointer hover:bg-element-hover hover:text-text active:bg-element-active focus:bg-element-hover'
                : 'flex flex-row items-center gap-2 h-7 px-2 pl-6 rounded-md text-ui-13 text-text/88 text-left cursor-pointer hover:bg-element-hover hover:text-text active:bg-element-active focus:bg-element-hover'}>
              <span className="size-3.5 flex-none text-text-muted">{renderIcon('bot')}</span>
              <span className="min-w-0 truncate whitespace-nowrap">{row.label}</span>
            </button>
          ))}
        </div>
      ))}
    </div>
  );
}
