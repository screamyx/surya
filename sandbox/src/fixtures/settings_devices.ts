// Fixture props for the Devices, Agents and Shortcuts settings pages.
// Rust models: surya_proto::entities::Device (app/crates/proto/src/entities.rs),
// surya_engine::registry::HarnessDescriptor (app/crates/engine/src/registry.rs),
// ShortcutId + KeymapConfig (app/crates/ui/src/settings.rs).
// The engine keeps owning the RPCs; these are plain records with the same ids.

// --- Devices ---------------------------------------------------------------

// `online`, `lastSeenLabel` and `addedLabel` are clock-derived in Rust
// (device_online / format_last_seen against Utc::now()). The sandbox has no
// clock, so the fixture carries the already-formatted values.
export type DeviceRow = {
  id: string;
  name: string;
  platform: string;
  version: string | null;
  online: boolean;
  lastSeenLabel: string;
  addedLabel: string | null;
};

export type WorkspaceScope = 'local' | 'synced' | 'development';

export const seededDevices: readonly DeviceRow[] = [
  { id: 'dev-8f3a91c40b27e5d6', name: 'dtry', platform: 'windows', version: '0.4.2',
    online: true, lastSeenLabel: 'just now', addedLabel: 'Added 41d ago' },
  { id: 'dev-2c7be04a19f8d3aa', name: 'devbox', platform: 'linux', version: '0.4.2',
    online: true, lastSeenLabel: 'just now', addedLabel: 'Added 41d ago' },
  { id: 'dev-55d1e7ab3c90f412', name: 'MacBook Pro', platform: 'macos', version: '0.4.1',
    online: false, lastSeenLabel: '3h ago', addedLabel: 'Added 12d ago' },
  { id: 'dev-a09fc6821d4e7b35', name: 'Pixel 8', platform: 'android', version: null,
    online: false, lastSeenLabel: 'never seen', addedLabel: 'Added 2d ago' },
];

export const localDeviceId = 'dev-2c7be04a19f8d3aa';

// --- Agents (harnesses) ----------------------------------------------------

export type HarnessRow = {
  id: string;
  name: string;
  blurb: string;
  cliName: string;
  installed: boolean;
  enabled: boolean;
};

export type LoadState = 'idle' | 'loading' | 'ready' | 'error';

// blurb and cliName are settings/agent_labels.rs, verbatim.
export const seededHarnesses: readonly HarnessRow[] = [
  { id: 'claudeCode', name: 'Claude Code', cliName: 'claude', installed: true, enabled: true,
    blurb: "Anthropic's coding agent, driven through the Claude Code CLI." },
  { id: 'codex', name: 'Codex', cliName: 'codex', installed: true, enabled: false,
    blurb: "OpenAI's coding agent, driven through the Codex CLI." },
  { id: 'cursor', name: 'Cursor', cliName: 'cursor-agent', installed: true, enabled: true,
    blurb: "Cursor's coding agent, driven through the cursor-agent CLI." },
  { id: 'devin', name: 'Devin', cliName: 'devin', installed: false, enabled: false,
    blurb: "Cognition's Devin agent (devin CLI)." },
  { id: 'grok', name: 'Grok', cliName: 'grok', installed: false, enabled: true,
    blurb: "xAI's Grok Build agent (grok CLI)." },
  { id: 'hermes', name: 'Hermes', cliName: 'hermes', installed: false, enabled: false,
    blurb: "Nous Research's Hermes Agent (hermes CLI)." },
  { id: 'pi', name: 'Pi', cliName: 'pi', installed: true, enabled: false,
    blurb: 'The pi coding agent (pi CLI).' },
  { id: 'opencode', name: 'opencode', cliName: 'opencode', installed: true, enabled: false,
    blurb: "SST's opencode agent (opencode CLI)." },
];

// --- Shortcuts -------------------------------------------------------------

// ShortcutId::ALL order, with label() and default_combo_on(false) from
// settings.rs. Ids are the camelCase KeymapConfig field names; the jump slots
// are the list index, as in KeymapConfig::jump_session.
export type ShortcutDef = { id: string; label: string; defaultCombo: string };

export const shortcutCatalog: readonly ShortcutDef[] = [
  { id: 'toggleSidebar', label: 'Toggle left sidebar', defaultCombo: 'mod-s' },
  { id: 'toggleChanges', label: 'Toggle right sidebar', defaultCombo: 'mod-b' },
  { id: 'toggleTerminal', label: 'Toggle terminal', defaultCombo: 'mod-j' },
  { id: 'toggleFiles', label: 'Toggle files pane', defaultCombo: 'mod-shift-f' },
  { id: 'toggleTasks', label: 'Toggle tasks pane', defaultCombo: 'mod-shift-t' },
  { id: 'toggleInbox', label: 'Toggle the needs-you list', defaultCombo: 'mod-shift-i' },
  { id: 'newSession', label: 'New session', defaultCombo: 'mod-n' },
  { id: 'nextSession', label: 'Next session', defaultCombo: 'mod-tab' },
  { id: 'prevSession', label: 'Previous session', defaultCombo: 'mod-shift-tab' },
  { id: 'archiveSession', label: 'Archive session', defaultCombo: 'mod-shift-a' },
  { id: 'jumpSession0', label: 'Jump to session 1', defaultCombo: 'mod-1' },
  { id: 'jumpSession1', label: 'Jump to session 2', defaultCombo: 'mod-2' },
  { id: 'jumpSession2', label: 'Jump to session 3', defaultCombo: 'mod-3' },
  { id: 'jumpSession3', label: 'Jump to session 4', defaultCombo: 'mod-4' },
  { id: 'jumpSession4', label: 'Jump to session 5', defaultCombo: 'mod-5' },
  { id: 'jumpSession5', label: 'Jump to session 6', defaultCombo: 'mod-6' },
  { id: 'jumpSession6', label: 'Jump to session 7', defaultCombo: 'mod-7' },
  { id: 'jumpSession7', label: 'Jump to session 8', defaultCombo: 'mod-8' },
  { id: 'jumpSession8', label: 'Jump to session 9', defaultCombo: 'mod-9' },
];

export type Keymap = Readonly<Record<string, string>>;

export const defaultKeymap: Keymap = Object.fromEntries(
  shortcutCatalog.map(entry => [entry.id, entry.defaultCombo]));

// One rebound row, so the seeded capture shows the Reset action and the
// enabled Restore defaults button.
export const seededKeymap: Keymap = { ...defaultKeymap, toggleTerminal: 'mod-alt-t' };
