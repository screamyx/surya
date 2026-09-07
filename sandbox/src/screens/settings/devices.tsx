// Rust: app/crates/ui/src/settings/devices.rs (DevicesPage::render, plus its
// render_rename_dialog impl helper, which stays a plain function here).
// The Rename mutation and the clipboard write stay in Rust; this page emits
// onRename(deviceId, name) and onCopyId(deviceId).
import { useState } from 'react';
import type { DeviceRow, WorkspaceScope } from '../../fixtures/settings_devices';
import { cardRow, errorStrip, metaLine, pageColumn, pageHeader, pageSubtitle,
  rowTile, rowTitle, sectionCard, settingsIcon } from './widgets';
import type { SettingsIcon } from './widgets';

export type DevicesProps = {
  devices: readonly DeviceRow[];
  localDeviceId: string | null;
  workspaceScope: WorkspaceScope;
  error?: string;
  onRename: (deviceId: string, name: string) => void;
  onCopyId: (deviceId: string) => void;
};

/// Rust: devices_subtitle(scope). Scope-aware copy, verbatim.
export function devicesSubtitle(scope: WorkspaceScope) {
  if (scope === 'local') return 'Manage device details stored in this local workspace.';
  if (scope === 'synced') return 'Manage device names and inspect synced device metadata.';
  return 'Manage device names for this workspace.';
}

/// Rust: platform_label(platform).
export function platformLabel(platform: string) {
  if (platform === 'macos' || platform === 'darwin') return 'macOS';
  if (platform === 'linux') return 'Linux';
  if (platform === 'windows') return 'Windows';
  if (platform === 'web') return 'Web';
  if (platform === 'ios') return 'iOS';
  if (platform === 'android') return 'Android';
  return platform;
}

/// Rust: short_id(id). The click-to-copy chip's `abcd1234...wxyz`.
export function shortId(id: string) {
  return id.length > 12 ? `${id.slice(0, 8)}…${id.slice(-4)}` : id;
}

function platformGlyph(platform: string): SettingsIcon {
  if (platform === 'macos' || platform === 'darwin') return 'laptop';
  if (platform === 'web') return 'global';
  if (platform === 'ios' || platform === 'android') return 'smartphone';
  return 'monitor';
}

/// Rust: the presence dot on the identity tile. The native dot sits at
/// bottom/right -3px with a 2px card-toned ring and an emerald glow; the
/// whitelist carries neither a negative offset, a 2px border nor a shadow, so
/// it sits flush in the tile's corner with a 1px surface ring.
function identityTile(platform: string, online: boolean) {
  return (
    <div className="flex-none relative flex">
      {rowTile(platformGlyph(platform))}
      <span className={online
        ? 'absolute bottom-0 right-0 size-2.25 rounded-full border border-surface bg-success'
        : 'absolute bottom-0 right-0 size-2.25 rounded-full border border-surface bg-wash/14'} />
    </div>
  );
}

/// Rust: the id chip fragment inside meta_line. Mono in Rust; the sandbox has
/// only the one generated font family.
function idChip(device: DeviceRow, copied: boolean, onCopyId: (id: string) => void) {
  return (
    <button key="id" type="button" onClick={() => onCopyId(device.id)}
      aria-label={`Copy device id for ${device.name}`} data-device-id={device.id}
      className={copied
        ? 'text-ui-10 text-success-muted/88 cursor-pointer'
        : 'text-ui-10 text-text-faint/50 cursor-pointer hover:text-text-muted active:text-text focus:text-text-muted'}>
      {copied ? 'Copied' : shortId(device.id)}
    </button>
  );
}

/// Rust: DevicesPage::render_rename_dialog. A plain render function on the
/// page, not a second component; popover::modal + dialog_card metrics.
function renameDialog(name: string, onName: (value: string) => void,
  onCancel: () => void, onSave: () => void) {
  return (
    <div className="absolute top-0 left-0 w-full h-full flex items-center justify-center bg-bg/50">
      <div role="dialog" aria-label="Rename device"
        className="w-96 p-5 rounded-xl border border-border bg-surface-dialog flex flex-col text-text">
        <div className="text-ui-14 font-semibold text-text">Rename device</div>
        <input value={name} onChange={event => onName(event.target.value)}
          aria-label="Device name" placeholder="Device name"
          className="mt-3 w-full min-w-0 px-3 py-2 rounded-lg border border-border bg-wash/5 text-ui-14 text-text" />
        <div className="mt-4 flex flex-row justify-end gap-2">
          <button type="button" onClick={onCancel}
            className="px-3 py-1.5 rounded-lg text-ui-13 text-text-muted cursor-pointer hover:bg-wash/5 hover:text-text active:bg-wash/10 focus:bg-wash/5 focus:text-text">Cancel</button>
          <button type="button" onClick={onSave}
            className="px-3 py-1.5 rounded-lg bg-text text-ui-13 font-medium text-on-solid cursor-pointer hover:bg-text/88 active:bg-text/50 focus:bg-text/88">Rename</button>
        </div>
      </div>
    </div>
  );
}

export function DevicesPage({ devices, localDeviceId, workspaceScope, error,
  onRename, onCopyId }: DevicesProps) {
  const [copied, setCopied] = useState('');
  const [dismissed, setDismissed] = useState(false);
  const [renaming, setRenaming] = useState('');
  const [draft, setDraft] = useState('');
  const count = devices.length;
  const strip = error && !dismissed
    ? errorStrip(error, () => setDismissed(true))
    : undefined;

  function copy(id: string) {
    setCopied(id);
    onCopyId(id);
  }

  function openRename(device: DeviceRow) {
    setRenaming(device.id);
    setDraft(device.name);
  }

  function submitRename() {
    const name = draft.trim();
    if (name) onRename(renaming, name);
    setRenaming('');
  }

  return (
    <div className="size-full overflow-y-scroll relative">
      {pageColumn(<>
        {pageHeader('Devices', count > 0 ? count : undefined)}
        {pageSubtitle(devicesSubtitle(workspaceScope))}
        {strip}
        {sectionCard(count === 0
          ? <div className="px-5 py-10 text-center text-ui-14 text-text-faint/50">No devices registered</div>
          : devices.map((device, index) => cardRow(index === 0, <>
            {identityTile(device.platform, device.online)}
            <div className="flex-1 min-w-0 flex flex-col">
              {rowTitle(device.name)}
              {metaLine([
                <span key="platform">{platformLabel(device.platform)}</span>,
                ...(device.version ? [<span key="version">{`v${device.version}`}</span>] : []),
                ...(device.online ? [] : [<span key="seen">{`Last seen ${device.lastSeenLabel}`}</span>]),
                ...(device.addedLabel ? [<span key="added">{device.addedLabel}</span>] : []),
                idChip(device, copied === device.id, copy),
              ])}
            </div>
            {localDeviceId === device.id && (
              <span className="flex-none text-ui-10 text-text-muted">
                {workspaceScope === 'local' ? 'Local only' : 'This device'}
              </span>
            )}
            <button type="button" onClick={() => openRename(device)}
              aria-label={`Rename ${device.name}`} data-action="Rename"
              className="flex-none flex flex-row items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-ui-12 text-text-muted cursor-pointer opacity-55 hover:opacity-100 hover:bg-wash/5 hover:text-text active:bg-wash/10 focus:opacity-100 focus:bg-wash/5 focus:text-text">
              <span className="flex-none size-3.5 flex">{settingsIcon('pen')}</span>
              Rename
            </button>
          </>))
        )}
      </>)}
      {renaming && renameDialog(draft, setDraft, () => setRenaming(''), submitRename)}
    </div>
  );
}
