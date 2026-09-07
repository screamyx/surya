// Rust: app/crates/ui/src/settings/harnesses.rs (HarnessesPage::render, its
// render_device_switcher and rows impl helpers, which stay plain functions
// here). Copy comes from settings/agent_labels.rs. ListHarnesses,
// SetHarnessEnabled and the targetDeviceId passthrough stay in Rust; this page
// emits onToggle(harnessId, enabled), onTargetDevice(deviceId) and onRetry.
import { useState } from 'react';
import type { DeviceRow, HarnessRow, LoadState } from '../../fixtures/settings_devices';
import { cardRow, errorStrip, metaLine, pageColumn, pageHeader, pageSubtitle,
  rowTitle, sectionCard, toggleSwitch } from './widgets';
import { yoloDefaultSection } from './harnesses/yolo_default';
import { renderIcon } from '../../icons';
import type { IconName } from '../../icons';

export type HarnessesProps = {
  harnesses: readonly HarnessRow[];
  state: LoadState;
  loadError?: string;
  toggleError?: string;
  devices: readonly DeviceRow[];
  localDeviceId: string | null;
  yoloDefault: boolean;
  onToggle: (harnessId: string, enabled: boolean) => void;
  onTargetDevice: (deviceId: string | null) => void;
  onRetry: () => void;
  onYoloDefault: (enabled: boolean) => void;
};

/// Rust: popover::tracked_upper('Devices') - uppercase, hair-space tracking.
const HEADING = 'D\u200AE\u200AV\u200AI\u200AC\u200AE\u200AS';

const SUBTITLE = 'Choose which coding agents the composer offers. The setting is per '
  + "device - switch devices in the header. Agents whose CLI isn't installed on a "
  + "device can't be enabled there.";

/// Rust: crate::pickers::platform_glyph inside render_device_switcher.
function platformGlyph(platform: string): IconName {
  if (platform === 'macos' || platform === 'darwin') return 'laptop';
  if (platform === 'ios' || platform === 'android') return 'smartphone';
  return 'monitor';
}

/// Rust: icons::SORT_VERTICAL, which the shared widget kit does not carry.
function sortVertical() {
  return (
    <svg viewBox="0 0 24 24" className="size-full" aria-hidden="true">
      <path fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"
        strokeLinejoin="round" d="M8 20V4m0 0L5 7.5M8 4l3 3.5M16 4v16m0 0l3-3.5M16 20l-3-3.5" />
    </svg>
  );
}

/// Rust: pickers::harness_brand_icon(harness). Only the Claude mark ships as a
/// sandbox asset; the other vendor marks fall back to one neutral agent glyph.
function harnessMark(id: string) {
  if (id === 'claudeCode' || id === 'mock') {
    return (
      <span className="size-4 flex text-claude-brand">
        <svg viewBox="0 0 24 24" className="size-full" aria-hidden="true">
          <path fill="currentColor" d="m4.7 15.9l4.6-2.6l.1-.2l-.1-.1H9l-.6-.1H6.5l-1.6-.1l-1.5-.1l-1.5-.2l-.4-.4l.1-.3l.4-.2h.5l1.2.1l1.7.1l1.3.1l1.8.2h.3v-.2l-.1-.1l-.1-.1l-2.2-1.5l-2.4-1.6l-1.2-.9l-.7-.5l-.3-.4l-.2-.9l.6-.7h.8l.2.1l.8.6l1.7 1.3l2.2 1.7l.4.2v-.2L8 8l-1-1.8l-1-1.9l-.5-.8l-.1-.5c0-.2 0-.4.2-.5l.3-.1l.7.1l.3.3l.4 1l.7 1.5l1 2l.3.6l.2.3v-.2l.2-1.9l.3-2.4l.2-1.4l.2-.8l.5-.5l.4-.1l.4.4l-.1.4l-.3 1.4l-.5 2.6l-.3 1.7h.2l.2-.2l.8-1.1l1.4-1.7l.6-.7l.7-.7l.5-.4h.9l.6 1l-.3.9l-.9 1.1l-.7 1l-1.1 1.4l-.7 1.2l.1.1h.2l3-.7l1.6-.3l2-.3l.8.4l.1.5l-.4.8l-2.2.5l-2.6.5l-3.9.9l-.1.1l.1.1l1.7.1h3.7l2.1.2l.6.4l-.3.7l-.6.3l-1-.2l-2.3-.6h-3.2l-.1.1v.1l1.1 1.1l2 1.8l2.5 2.3l.1.6l-.3.4h-.4l-2.1-1.6l-.8-.7l-1.9-1.6h-.1v.2l.4.6l2.3 3.5l.1 1.1l-.2.3l-.6-.1l-1.2-1.7l-1.2-1.9l-1-1.7l-.1.1l-.6 6.3l-.3.3l-.6-.3l-.5-1.2l.5-2.4l.6-3.1l.5-2.5v-.4h-.1l-1 1.3l-1.5 2l-1.2 1.3l-.3.1l-.5-.5l.1-.5l.3-.4l1.7-2.2l1-1.4l.7-.8v-.3h-.1L5 17.2l-1.4.2l-.6-.6l.1-.4l.3-.3z" />
        </svg>
      </span>
    );
  }
  return (
    <span className="size-4 flex text-text-muted">
      <svg viewBox="0 0 24 24" className="size-full" aria-hidden="true">
        <g fill="none" stroke="currentColor" strokeWidth="1.5">
          <path d="M4 13c0-2.828 0-4.243.879-5.121C5.757 7 7.172 7 10 7h4c2.828 0 4.243 0 5.121.879C20 8.757 20 10.172 20 13s0 4.243-.879 5.121C18.243 19 16.828 19 14 19h-4c-2.828 0-4.243 0-5.121-.879C4 17.243 4 15.828 4 13Z" />
          <path strokeLinecap="round" d="M12 7V4M2 13h2m16 0h2" />
          <path strokeLinecap="round" d="M9.5 13.5v-1m5 1v-1" />
        </g>
      </svg>
    </span>
  );
}

/// Rust: the harness row tile. widgets::rowTile takes a named kit icon, and the
/// brand marks are not in the kit, so the tile is spelled out here.
function harnessTile(id: string) {
  return (
    <div className="flex-none size-9 rounded-lg border border-border bg-wash/5 flex items-center justify-center">
      {harnessMark(id)}
    </div>
  );
}

/// Rust: HarnessesPage::render_device_switcher, trigger half. Platform glyph,
/// name, presence dot and sort glyph.
function deviceSwitcherTrigger(devices: readonly DeviceRow[], localDeviceId: string | null,
  target: string | null, open: boolean, onOpen: () => void) {
  const effective = target ?? localDeviceId;
  const selected = devices.find(device => device.id === effective);
  return (
    <button type="button" onClick={onOpen} aria-expanded={open}
      aria-label="Switch device" data-action="Switch device"
      className={open
        ? 'flex-none h-7 px-2 rounded-md flex flex-row items-center gap-1.5 cursor-pointer bg-wash/5 text-text'
        : 'flex-none h-7 px-2 rounded-md flex flex-row items-center gap-1.5 cursor-pointer hover:bg-wash/5 active:bg-wash/10 focus:bg-wash/5 text-text'}>
      <span className="flex-none size-4 flex text-text-muted">
        {renderIcon(platformGlyph(selected ? selected.platform : 'macos'))}
      </span>
      <span className="min-w-0 truncate text-ui-12 font-medium text-text">
        {selected ? selected.name : 'This device'}
      </span>
      <span className={effective === localDeviceId
        ? 'flex-none size-1.5 rounded-full bg-success'
        : 'flex-none size-1.5 rounded-full bg-wash/14'} />
      <span className={open
        ? 'flex-none size-3.5 flex text-text-muted'
        : 'flex-none size-3.5 flex text-text-faint/50'}>{sortVertical()}</span>
    </button>
  );
}

/// Rust: the popover::anchored_menu half of render_device_switcher. The native
/// menu is a deferred, occluding layer above everything; the sandbox has no
/// z-index, so the page renders it last inside a relative column and it is
/// anchored to that column's top right rather than nested in the trigger.
function deviceMenu(devices: readonly DeviceRow[], localDeviceId: string | null,
  target: string | null, onPick: (deviceId: string | null) => void) {
  const effective = target ?? localDeviceId;
  return (
    <div className="absolute top-8 right-0">
    <div role="menu" aria-label="Devices"
      className="relative motion-menu-in w-55 p-1 rounded-xl border border-border bg-surface-overlay text-ui-13 text-text flex flex-col gap-0.5">
      <div className="px-2 pt-1.5 pb-1 text-ui-10 font-medium text-text-faint">{HEADING}</div>
      {devices.map(device => (
        <button key={device.id} type="button" role="menuitem"
          onClick={() => onPick(device.id === localDeviceId ? null : device.id)}
          data-device={device.name}
          className={device.id === effective
            ? 'flex flex-row items-center gap-2.5 px-2 py-1.5 rounded-lg text-ui-13 text-text cursor-pointer bg-wash/10 text-left'
            : 'motion-hover-fade flex flex-row items-center gap-2.5 px-2 py-1.5 rounded-lg text-ui-13 text-text cursor-pointer hover:bg-wash/10 active:bg-wash/14 focus:bg-wash/10 text-left'}>
          <span className="flex-none size-4 flex text-text-muted">
            {renderIcon(platformGlyph(device.platform))}
          </span>
          <span className="flex-1 min-w-0 truncate">{device.name}</span>
          {device.id === localDeviceId && <span className="flex-none text-ui-10 text-text-faint/50">You</span>}
          <span className={device.id === localDeviceId
            ? 'flex-none size-1.5 rounded-full bg-success'
            : 'flex-none size-1.5 rounded-full bg-wash/14'} />
        </button>
      ))}
    </div>
    </div>
  );
}

/// Rust: popover::skeleton_rows. The native bars pulse on SURYA_PULSE at
/// opacity 0.35 + 0.4 * wave, staggered 0.08 per bar and with no size change.
/// motion-surya-pulse-* is the loaders.rs cell, which runs 0.08 to 1 and 90% to
/// 100% at a 0.0625 stagger, so it would paint a different animation. The bars
/// rest at one opacity until the catalog carries this pulse.
function skeletonRows(count: number) {
  return (
    <div className="p-4 flex flex-col">
      <div className="flex flex-col gap-1.5 py-1">
        {Array.from({ length: count }, (_unused, index) => (
          <div key={index} className="h-7 rounded-md bg-wash/5 opacity-55" />
        ))}
      </div>
    </div>
  );
}

/// Rust: HarnessesPage::rows. `interactive` mirrors the engine guard: the last
/// enabled installed agent cannot be switched off, and an agent whose CLI is
/// missing can only be switched off.
function harnessRows(harnesses: readonly HarnessRow[], enabled: Readonly<Record<string, boolean>>,
  onToggle: (id: string, next: boolean) => void) {
  const enabledCount = harnesses.filter(row => enabled[row.id]).length;
  return harnesses.map((row, index) => {
    const on = enabled[row.id];
    const lastEnabled = on && enabledCount === 1 && row.installed;
    const interactive = !lastEnabled && (on || row.installed);
    return cardRow(index === 0, <>
      {harnessTile(row.id)}
      <div className="flex-1 min-w-0 flex flex-col">
        {rowTitle(row.name)}
        {metaLine([
          <span key="blurb">{row.blurb}</span>,
          ...(row.installed ? [] : [
            <span key="hint" className="text-warning-muted/88">
              {on
                ? `${row.cliName} CLI not installed - turn it off or install it`
                : `Install the ${row.cliName} CLI to enable`}
            </span>,
          ]),
        ])}
      </div>
      {toggleSwitch(on, row.name, () => onToggle(row.id, !on), !interactive)}
    </>, !row.installed);
  });
}

export function HarnessesPage({ harnesses, state, loadError, toggleError, devices,
  localDeviceId, yoloDefault, onToggle, onTargetDevice, onRetry,
  onYoloDefault }: HarnessesProps) {
  const [enabled, setEnabled] = useState<Readonly<Record<string, boolean>>>(
    Object.fromEntries(harnesses.map(row => [row.id, row.enabled])));
  const [target, setTarget] = useState<string | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  const [yolo, setYolo] = useState(yoloDefault);

  function toggle(id: string, next: boolean) {
    setEnabled({ ...enabled, [id]: next });
    onToggle(id, next);
  }

  function pick(deviceId: string | null) {
    setMenuOpen(false);
    setTarget(deviceId);
    onTargetDevice(deviceId);
  }

  let body;
  if (state === 'idle' || state === 'loading') body = sectionCard(skeletonRows(4));
  else if (state === 'error') {
    body = (
      <div>
        {errorStrip(loadError ?? 'The engine did not answer.')}
        <div className="mt-2 flex flex-row">
          <button type="button" onClick={onRetry} data-action="Retry"
            className="flex-none flex flex-row items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-ui-12 text-text-muted cursor-pointer hover:bg-wash/5 hover:text-text active:bg-wash/10 focus:bg-wash/5 focus:text-text">
            <span className="flex-none size-3.5 flex">{renderIcon('refresh')}</span>
            Retry
          </button>
        </div>
      </div>
    );
  } else body = sectionCard(harnessRows(harnesses, enabled, toggle));

  return (
    <div className="size-full overflow-y-scroll">
      {pageColumn(
        <div className="relative flex flex-col">
          <div className="flex flex-row items-center justify-between">
            {pageHeader('Agents')}
            {deviceSwitcherTrigger(devices, localDeviceId, target, menuOpen,
              () => setMenuOpen(!menuOpen))}
          </div>
          {pageSubtitle(SUBTITLE, true)}
          {toggleError && errorStrip(toggleError)}
          {body}
          {yoloDefaultSection(yolo, () => { setYolo(!yolo); onYoloDefault(!yolo); })}
          {menuOpen && deviceMenu(devices, localDeviceId, target, pick)}
        </div>)}
    </div>
  );
}
