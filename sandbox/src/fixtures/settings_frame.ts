// Fixture props for the settings window frame, Notifications, Archived and
// Servers. The native engine keeps subscriptions, RPC and persistence.
import type { NotificationFlags } from '../screens/settings/notifications';
import type { ArchivedRow } from '../screens/settings/archived';
import type { ServerRow, ActiveRow } from '../screens/settings/servers';

// Rust: shell.rs SettingsSection::ALL and SettingsSection::label().
export const settingsSections = [
  { id: 'devices', label: 'Devices', icon: 'monitor' },
  { id: 'servers', label: 'Servers', icon: 'global' },
  { id: 'harnesses', label: 'Agents', icon: 'widget' },
  { id: 'agents', label: 'Accounts', icon: 'key-minimalistic' },
  { id: 'appearance', label: 'Appearance', icon: 'tuning' },
  { id: 'notifications', label: 'Notifications', icon: 'bell' },
  { id: 'shortcuts', label: 'Shortcuts', icon: 'keyboard' },
  { id: 'archived', label: 'Archived sessions', icon: 'archive-minimalistic' },
] as const;

export const seededNotifications: NotificationFlags = { sound: true, desktop: true, backgroundOnly: false };
export const emptyNotifications: NotificationFlags = { sound: false, desktop: false, backgroundOnly: false };

export const seededArchived: readonly ArchivedRow[] = [
  { id: 'chat-dock-row-tone', title: 'Dock Row Tone Feature Work', device: 'devbox', location: 'luvus', timeAgo: '4h' },
  { id: 'chat-optional-tone', title: 'Optional Tone And Spans', device: 'devbox', location: 'luvus', timeAgo: '14h' },
  { id: 'chat-leads-table', title: 'Display All Leads Table', device: 'buildbox', location: null, timeAgo: '15h' },
  { id: 'chat-untitled', title: 'Untitled session', device: null, location: 'Downloads', timeAgo: '19h' },
];

export const seededServers: readonly ServerRow[] = [
  { id: 'srv-build', name: 'Build box', host: '100.64.0.9', port: 27700, tokenSet: true },
  { id: 'srv-attic', name: 'Attic', host: 'attic.local', port: 27700, tokenSet: false },
];

export const seededActiveRow: ActiveRow = { kind: 'saved', id: 'srv-build' };
export const localActiveRow: ActiveRow = { kind: 'local' };

// Rust: settings/servers/active.rs status_text().
export const seededStatus = { text: 'Connected to Build box (ws://100.64.0.9:27700)', isError: false };
export const emptyStatus = { text: 'Using the engine on this computer', isError: false };
export const failedStatus = { text: 'Not connected: connection refused', isError: true };
