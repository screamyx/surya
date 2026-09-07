// Fixture props for the Accounts and Appearance settings pages. The native
// engine keeps every RPC, subscription and file read; these are plain records
// shaped like the Rust models so the harness can render both screens.
// Rust: app/crates/ui/src/settings/accounts.rs, app/crates/ui/src/settings/appearance.rs.
import type { AccountsProps, AgentAccount, DeviceRow, ProviderId } from '../screens/settings/accounts';
import type { AppearanceProps, CustomThemeEntry, ThemeVariantOption } from '../screens/settings/appearance';

// ---------------------------------------------------------------------------
// Accounts
// ---------------------------------------------------------------------------

const claudeAccounts: readonly AgentAccount[] = [
  {
    id: 'claude-live', harness: 'claude-code', email: 'ada@example.com', planLabel: 'Max 20x',
    active: true, switchable: true,
    usageWindows: [
      { label: '5h', usedFraction: 0.42, reset: 'resets 3:45 PM' },
      { label: 'Week', usedFraction: 0.83, reset: 'resets Mon' },
    ],
  },
  {
    id: 'claude-spare', harness: 'claude-code', email: 'grace@example.com', planLabel: 'Pro',
    active: false, switchable: true,
    usageWindows: [
      { label: '5h', usedFraction: 0.97, reset: 'resets 11:10 AM' },
      { label: 'Week', usedFraction: 0.08, reset: 'resets Sep 14' },
    ],
  },
  {
    id: 'claude-stale', harness: 'claude-code', email: 'alan@example.com',
    active: false, switchable: false, usageWindows: [],
  },
];

const codexAccounts: readonly AgentAccount[] = [
  {
    id: 'codex-live', harness: 'codex', email: 'ada@example.com', planLabel: 'Plus',
    active: true, switchable: true,
    usageWindows: [{ label: 'Month', usedFraction: 0.15, reset: 'resets Oct 1' }],
  },
];

export const seededAccounts: AccountsProps = {
  status: 'ready',
  accounts: [...claudeAccounts, ...codexAccounts],
  warnings: [{ harness: 'codex', message: 'The Codex profile directory is not readable, so usage may be stale.' }],
  devices: [
    { id: 'device-dtry', name: 'dtry', platform: 'windows' },
    { id: 'device-book', name: 'Ada Book', platform: 'macos' },
    { id: 'device-phone', name: 'Ada Phone', platform: 'ios' },
  ],
  localDeviceId: 'device-dtry',
  busyAccountId: undefined,
  actionError: undefined,
  loadError: undefined,
  login: undefined,
};

export const emptyAccounts: AccountsProps = {
  ...seededAccounts, accounts: [], warnings: [],
};

export const loadingAccounts: AccountsProps = { ...emptyAccounts, status: 'loading' };

export const failedAccounts: AccountsProps = {
  ...emptyAccounts, status: 'error',
  loadError: 'The engine did not answer ListAgentAccounts.',
};

export const busyAccounts: AccountsProps = {
  ...seededAccounts, busyAccountId: 'claude-spare',
  actionError: 'Switching to grace@example.com failed: the CLI holds the login.',
};

export const pasteCodeAccounts: AccountsProps = {
  ...seededAccounts,
  login: {
    kind: 'pasteCode', harness: 'claude-code', url: 'https://claude.ai/oauth/authorize',
    code: '', submitting: false, error: undefined,
  },
};

export const browserLoginAccounts: AccountsProps = {
  ...seededAccounts,
  login: { kind: 'browser', harness: 'codex', url: 'https://auth.openai.com/authorize', message: undefined, error: undefined },
};

export const providerCliName: Record<ProviderId, string> = {
  'claude-code': 'claude', codex: 'codex', cursor: 'cursor-agent',
};

export const extraDevice: DeviceRow = { id: 'device-rig', name: 'Build rig', platform: 'linux' };

// ---------------------------------------------------------------------------
// Appearance
// ---------------------------------------------------------------------------

export const lightVariants: readonly ThemeVariantOption[] = [
  { id: 'surya-light', name: 'Surya Light', appearance: 'light' },
  { id: 'zeron-light', name: 'Zeron Light', appearance: 'light' },
  { id: 'catppuccin-latte', name: 'Catppuccin Latte', appearance: 'light' },
  { id: 'github-light', name: 'GitHub Light', appearance: 'light' },
  { id: 'ayu-light', name: 'Ayu Light', appearance: 'light' },
];

export const darkVariants: readonly ThemeVariantOption[] = [
  { id: 'surya-dark', name: 'Surya Dark', appearance: 'dark' },
  { id: 'zeron-dark', name: 'Zeron Dark', appearance: 'dark' },
  { id: 'tokyo-night', name: 'Tokyo Night', appearance: 'dark' },
  { id: 'dracula', name: 'Dracula', appearance: 'dark' },
  { id: 'one-dark-pro', name: 'One Dark Pro', appearance: 'dark' },
];

const libraryEntries: readonly CustomThemeEntry[] = [
  {
    id: 'night-owl', name: 'Night Owl', linked: false, status: 'ready',
    statusLine: 'Imported copy · 2 variants · Self-contained snapshot',
    variants: [
      { id: 'night-owl-dark', name: 'Night Owl', appearance: 'dark' },
      { id: 'night-owl-light', name: 'Night Owl Light', appearance: 'light' },
    ],
    reports: {
      'night-owl-dark': {
        summary: '212 mapped · 6 adjusted · 3 inferred/fallback · 1 unsupported · 2 warnings · 0 validation',
        lines: [
          'Adjusted · text #011627 to #d6deeb · contrast against surface was 1.4:1',
          'Fallback · terminal.ansiBrightBlack was inferred from ansiBlack',
          'Warning · editorGroupHeader.tabsBackground has no Surya role',
          'Unsupported · editorGhostText.foreground',
        ],
      },
      'night-owl-light': {
        summary: '205 mapped · 2 adjusted · 9 inferred/fallback · 1 unsupported · 0 warnings · 0 validation',
        lines: ['Adjusted · border #d9d9d9 to #c4c4c4 · hairline was invisible on white'],
      },
    },
  },
  {
    id: 'work-in-progress', name: 'Work in progress', linked: true, status: 'warning',
    statusLine: 'Using last known good · the file on disk no longer parses',
    variants: [{ id: 'wip-dark', name: 'Work in progress', appearance: 'dark' }],
    reports: {
      'wip-dark': {
        summary: '188 mapped · 0 adjusted · 14 inferred/fallback · 0 unsupported · 1 warnings · 1 validation',
        lines: ['Validation Contrast Warning · text over surface is 3.9:1, below the 4.5:1 floor'],
      },
    },
  },
];

export const seededAppearance: AppearanceProps = {
  mode: 'system',
  themes: { light: 'surya-light', dark: 'surya-dark' },
  lightVariants, darkVariants,
  accent: 'theme-default',
  surface: 'theme-default',
  resolvedSurface: 'frosted',
  motion: 'system',
  systemReducedMotion: false,
  font: 'Geist',
  fontChoices: [
    { label: 'Geist', available: true },
    { label: 'Geist Mono', available: true },
    { label: 'System UI', available: true },
    { label: 'Inter', available: false },
  ],
  fontSize: '16 px',
  fontSizes: ['12 px', '13 px', '14 px', '15 px', '16 px', '18 px', '20 px'],
  fontFailed: false,
  library: libraryEntries,
  libraryWarning: undefined,
};

export const emptyAppearance: AppearanceProps = {
  ...seededAppearance, library: [],
};

export const failedAppearance: AppearanceProps = {
  ...seededAppearance, fontFailed: true,
  libraryWarning: 'One linked theme could not be read at start-up and is using its last known good copy.',
};
