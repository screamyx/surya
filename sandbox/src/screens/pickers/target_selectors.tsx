// Rust: app/crates/ui/src/pickers.rs - render_target_selectors (2371). The
// new-session target row: device and project chips ABOVE the composer pill,
// left-aligned like the checkout toolbar. The menus open upward.
import type { ReactNode } from 'react';
import { footerChip, overlayAboveFooter } from './chips';

export type TargetSelectorsProps = {
  deviceLabel: string; projectLabel: string; deviceOffline: boolean;
  openKind: string | null;
  onToggleDevice: () => void; onToggleSpace: () => void;
  devicePopover: ReactNode; spacePopover: ReactNode;
};

export function renderTargetSelectors(props: TargetSelectorsProps) {
  return (
    <div className="w-full flex flex-row items-center gap-1 px-2.5">
      <div className="relative flex flex-row items-center min-w-0">
        {footerChip('picker-device', 'monitor', props.deviceLabel,
          props.openKind === 'device', props.deviceOffline, props.onToggleDevice)}
        {props.openKind === 'device' && overlayAboveFooter('start', props.devicePopover)}
      </div>
      <div className="relative flex flex-row items-center min-w-0">
        {footerChip('picker-project', 'folder', props.projectLabel,
          props.openKind === 'space', false, props.onToggleSpace)}
        {props.openKind === 'space' && overlayAboveFooter('start', props.spacePopover)}
      </div>
    </div>
  );
}
