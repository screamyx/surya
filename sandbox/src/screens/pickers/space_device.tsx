// Rust: app/crates/ui/src/pickers.rs - render_device_popover (1900) and
// render_space_popover (1984). Both are search over a device-scoped list; the
// project popover adds a "New project…" action row under a hairline.
import { icon, menuRowNav, popoverNote, searchBox } from './popover';
import type { Device, Space } from '../../fixtures/pickers';

export type DevicePopoverProps = {
  rows: readonly Device[]; query: string; selected: string | null; active: number;
  onPick: (id: string) => void; onHover: (index: number) => void;
};

export function renderDevicePopover(props: DevicePopoverProps) {
  const body = props.rows.length === 0
    ? popoverNote('No devices match.')
    : (
      <div className="flex flex-col gap-0.5 max-h-55 overflow-y-scroll">
        {props.rows.map((device, ix) => menuRowNav(
          `device-row-${ix}`, props.selected === device.id, ix === props.active,
          () => props.onPick(device.id), () => props.onHover(ix),
          <>
            <span className="flex-1 min-w-0 truncate">{device.name}</span>
            {device.local && <span className="flex-none text-ui-10 text-text-faint">You</span>}
            {!device.online && <span className="flex-none text-warning">{icon('wifi-off', 12)}</span>}
          </>,
        ))}
      </div>
    );
  return <div className="flex flex-col">{searchBox(props.query, 'Search…')}{body}</div>;
}

export type SpacePopoverProps = {
  rows: readonly Space[]; query: string; selected: string | null; active: number;
  onPick: (id: string) => void; onHover: (index: number) => void; onNewProject: () => void;
};

export function renderSpacePopover(props: SpacePopoverProps) {
  // Distinguish "the filter ate everything" from "this device has no projects
  // yet" - the scoped list makes the latter common.
  const body = props.rows.length === 0
    ? popoverNote(props.query ? 'No projects match.' : 'No projects on this device.')
    : (
      <div className="flex flex-col gap-0.5 max-h-55 overflow-y-scroll">
        {props.rows.map((space, ix) => menuRowNav(
          `space-row-${ix}`, props.selected === space.id, ix === props.active,
          () => props.onPick(space.id), () => props.onHover(ix),
          <span className="flex-1 min-w-0 truncate">{space.name}</span>,
        ))}
      </div>
    );
  return (
    <div className="flex flex-col gap-0.5">
      {searchBox(props.query, 'Search…')}
      {body}
      <div className="my-0.5 h-0.25 flex-none bg-border" />
      {menuRowNav('project-new', false, false, props.onNewProject, () => undefined,
        <>
          <span className="flex-none text-text-faint">{icon('plus', 12)}</span>
          <span className="flex-1 min-w-0 truncate">New project…</span>
        </>)}
    </div>
  );
}
