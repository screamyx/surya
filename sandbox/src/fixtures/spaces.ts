// Fixture props for the sidebar spaces screen. Shapes follow the native model
// read by app/crates/ui/src/shell/spaces.rs: surya_proto::Chat (id, title,
// space_id, device_id, archived), surya_proto::Space (display_name, device_id),
// ChatIndicator, and surya_proto::ChangeRequestSummary. The engine keeps the
// subscriptions, sorting and transport; these are plain records.

/// surya_proto::ChatIndicator. The dot color and status word per variant are
/// in spaces.rs status_dot_color and shell.rs render_chat_row.
export type ChatStatus = 'working' | 'awaiting-input' | 'errored' | 'completed' | 'idle';

/// Send-truth overrides read in render_chat_row: an undelivered send is
/// Failed, a degraded one is Queued, and neither is the chat's own indicator.
export type SendState = 'normal' | 'queued' | 'undelivered';

/// crate::pickers::harness_brand_icon. Only the marks the sandbox carries.
export type HarnessMark = 'claude' | 'openai' | 'none';

/// surya_proto::ChangeRequestState, with its badge tone in change_requests.rs.
export type ChangeRequest = { number: string; state: 'open' | 'merged' | 'closed' };

export type ChatRow = {
  id: string;
  title: string;
  /// "project @ device" as render_active_rows builds it; project-less
  /// sessions read as their home-dir cwd.
  folder: string;
  /// Device group key when the sidebar organizes by device.
  deviceId: string;
  deviceName: string;
  timeAgo: string;
  branch: string | null;
  changeRequest: ChangeRequest | null;
  harness: HarnessMark;
  status: ChatStatus;
  send: SendState;
  archived: boolean;
};

export type SpaceRow = {
  id: string;
  name: string;
  /// AppState::space_device_tag: the "@ device" tag and whether it is offline.
  deviceTag: string;
  offline: boolean;
};

export type SidebarOrganization = 'by-device' | 'in-one-list';
export type SidebarSort = 'last-updated' | 'created';

/// UiSettings fields render_sidebar_view_menu reads and writes.
export type SidebarView = {
  organization: SidebarOrganization;
  sort: SidebarSort;
  showBranch: boolean;
  showPullRequest: boolean;
  showHarness: boolean;
};

export const defaultSidebarView: SidebarView = {
  organization: 'by-device',
  sort: 'last-updated',
  showBranch: true,
  showPullRequest: true,
  showHarness: true,
};

export const seededSpaces: readonly SpaceRow[] = [
  { id: 'surya', name: 'surya', deviceTag: '@ dtry', offline: false },
  { id: 'luvus', name: 'luvus', deviceTag: '@ devbox', offline: false },
  { id: 'agb', name: 'agb', deviceTag: '@ buildbox', offline: true },
];

export const seededChats: readonly ChatRow[] = [
  {
    id: 'wire-tasks', title: 'Wire the Tasks pane into the shell',
    folder: 'surya @ dtry', deviceId: 'dtry', deviceName: 'dtry', timeAgo: '2m',
    branch: 'wire-tasks-pane', changeRequest: { number: '266', state: 'open' },
    harness: 'claude', status: 'awaiting-input', send: 'normal', archived: false,
  },
  {
    id: 'folder-filter', title: 'The folder filter searches the whole folder',
    folder: 'surya @ dtry', deviceId: 'dtry', deviceName: 'dtry', timeAgo: '11m',
    branch: 'fix-folder-filter', changeRequest: null,
    harness: 'claude', status: 'working', send: 'normal', archived: false,
  },
  {
    id: 'fold-text-dim', title: 'Fold text_dim, a text tier that stopped being real',
    folder: 'surya @ dtry', deviceId: 'dtry', deviceName: 'dtry', timeAgo: '1h',
    branch: 'fold-text-dim', changeRequest: { number: '183', state: 'merged' },
    harness: 'claude', status: 'completed', send: 'normal', archived: false,
  },
  {
    id: 'queued-send', title: 'Rewrite the drive listing cache',
    folder: 'agb @ buildbox', deviceId: 'buildbox', deviceName: 'buildbox', timeAgo: '3h',
    branch: null, changeRequest: null,
    harness: 'openai', status: 'idle', send: 'queued', archived: false,
  },
  {
    id: 'failed-send', title: 'Port the composer footer target tag',
    folder: 'agb @ buildbox', deviceId: 'buildbox', deviceName: 'buildbox', timeAgo: '5h',
    branch: 'composer-footer', changeRequest: { number: '141', state: 'closed' },
    harness: 'none', status: 'errored', send: 'undelivered', archived: false,
  },
  {
    id: 'dock-row-tone', title: 'Dock row tone feature work',
    folder: 'luvus @ devbox', deviceId: 'devbox', deviceName: 'devbox', timeAgo: '14h',
    branch: 'dock-row-tone', changeRequest: null,
    harness: 'none', status: 'idle', send: 'normal', archived: false,
  },
  {
    id: 'leads-table', title: 'Display all leads table',
    folder: '~', deviceId: 'devbox', deviceName: 'devbox', timeAgo: '15h',
    branch: null, changeRequest: null,
    harness: 'none', status: 'idle', send: 'normal', archived: false,
  },
  {
    id: 'archived-tone', title: 'Optional tone and spans',
    folder: 'luvus @ devbox', deviceId: 'devbox', deviceName: 'devbox', timeAgo: '2d',
    branch: null, changeRequest: null,
    harness: 'claude', status: 'idle', send: 'normal', archived: true,
  },
  {
    id: 'archived-env', title: 'Local environment configuration',
    folder: 'surya @ dtry', deviceId: 'dtry', deviceName: 'dtry', timeAgo: '4d',
    branch: null, changeRequest: null,
    harness: 'openai', status: 'idle', send: 'normal', archived: true,
  },
  {
    id: 'archived-drives', title: 'Load the space drives once per device',
    folder: 'agb @ buildbox', deviceId: 'buildbox', deviceName: 'buildbox', timeAgo: '9d',
    branch: null, changeRequest: null,
    harness: 'none', status: 'idle', send: 'normal', archived: true,
  },
];

/// The right pane's surface tabs (shell.rs right_surface_rows).
export type SurfaceTab = { key: string; title: string; icon: 'git-branch' | 'bot' | 'folder' | 'checklist' | 'terminal' };

export const seededSurfaceTabs: readonly SurfaceTab[] = [
  { key: 'diff', title: 'Changes', icon: 'git-branch' },
  { key: 'files', title: 'Files', icon: 'folder' },
  { key: 'subagent', title: 'sbx-spaces', icon: 'bot' },
  { key: 'terminal', title: 'Terminal', icon: 'terminal' },
];

/// The agent tree rows the Agents section wraps. The tree itself is
/// inbox/agents.rs and belongs to another screen; these stand in for it.
export const seededAgentRows: readonly { id: string; label: string; depth: number }[] = [
  { id: 'jag', label: 'jag-0907-maroon', depth: 0 },
  { id: 'spaces', label: 'sbx-spaces', depth: 1 },
  { id: 'pickers', label: 'sbx-pickers', depth: 1 },
];
