// Fixture props for the Pickers screen. Rust model: app/crates/ui/src/pickers.rs
// (DraftConfig, PickerKind, ModelRowData) over surya_proto's Model, RepoRef,
// Space and Device. The native engine keeps ownership of loads and transport.
export type HarnessId = 'claude-code' | 'opencode' | 'cursor';
export type Harness = { id: HarnessId; name: string };
export type ReasoningLevel = 'Minimal' | 'Low' | 'Medium' | 'High' | 'X-High' | 'Max';
export type ModelChoice = { id: string; label: string };
export type ModelOption = {
  id: string; label: string; defaultChoice: string; choices: readonly ModelChoice[];
};
export type PickerModel = {
  id: string; label: string; description?: string;
  ladder: readonly ReasoningLevel[]; options: readonly ModelOption[];
};
export type RepoRef = { name: string; current: boolean; worktree: boolean };
export type Space = { id: string; name: string; gitDetected: boolean };
export type Device = { id: string; name: string; online: boolean; local: boolean };
export type CheckoutKind = 'local' | 'new-worktree';
export type YoloState = 'off' | 'on' | 'always';

/// One flattened model row (Rust `ModelRowData`): the model plus the harness
/// it belongs to, because search and the favorites tab mix harnesses.
export type ModelRow = { harness: HarnessId; harnessName: string; model: PickerModel };

export type PickersFixture = {
  harnesses: readonly Harness[];
  models: Readonly<Record<HarnessId, readonly PickerModel[]>>;
  refs: readonly RepoRef[];
  spaces: readonly Space[];
  devices: readonly Device[];
  favorites: readonly string[];
  selectedHarness: HarnessId | null;
  selectedModelId: string | null;
  selectedSpaceId: string | null;
  selectedDeviceId: string | null;
  branch: string | null;
  checkout: CheckoutKind;
  yolo: YoloState;
  /// Painted into the GPUI-owned search field; there is no browser input here.
  query: string;
  /// Mid-session SwitchRef failure, shown under a hairline in the ref popover.
  switchError: string | null;
  /// Total refs before MAX_REF_ROWS caps the list (Rust `MAX_REF_ROWS = 300`).
  refTotal: number;
};

const reasoning: readonly ReasoningLevel[] = ['Low', 'Medium', 'High', 'X-High'];

const contextOption: ModelOption = {
  id: 'context', label: 'Context window', defaultChoice: 'standard',
  choices: [{ id: 'standard', label: 'Standard' }, { id: '1m', label: '1M' }],
};
const speedOption: ModelOption = {
  id: 'speed', label: 'Speed', defaultChoice: 'balanced',
  choices: [{ id: 'balanced', label: 'Balanced' }, { id: 'fast', label: 'Fast' }],
};

export const seededPickers: PickersFixture = {
  harnesses: [
    { id: 'claude-code', name: 'Claude Code' },
    { id: 'opencode', name: 'opencode' },
    { id: 'cursor', name: 'Cursor' },
  ],
  models: {
    'claude-code': [
      { id: 'claude-opus-5', label: 'Opus 5', ladder: reasoning, options: [contextOption, speedOption] },
      { id: 'claude-sonnet-5', label: 'Sonnet 5', ladder: reasoning, options: [contextOption] },
      { id: 'claude-haiku-4-5', label: 'Haiku 4.5', ladder: [], options: [speedOption] },
      { id: 'claude-fable-5-1', label: 'Fable 5.1', ladder: reasoning, options: [] },
    ],
    'opencode': [
      { id: 'glm-5.2', label: 'GLM-5.2', description: 'zhipuai', ladder: ['Low', 'Medium'], options: [] },
      { id: 'glm-5.2-air', label: 'GLM-5.2', description: 'openrouter', ladder: ['Low', 'Medium'], options: [] },
      { id: 'kimi-k3', label: 'Kimi K3', description: 'moonshot', ladder: [], options: [] },
    ],
    'cursor': [
      { id: 'cursor-agent', label: 'Agent', description: 'Cursor', ladder: ['Low', 'Medium', 'High'], options: [speedOption] },
      { id: 'cursor-composer', label: 'Composer', ladder: [], options: [] },
    ],
  },
  refs: [
    { name: 'main', current: true, worktree: false },
    { name: 'sandbox/port-in-pickers', current: false, worktree: true },
    { name: 'fix/folder-filter-searches-the-whole-folder', current: false, worktree: false },
    { name: 'fix/picker-names-ctrl-on-windows', current: false, worktree: false },
    { name: 'feat/traits-tray', current: false, worktree: false },
    { name: 'release/2026-09', current: false, worktree: false },
  ],
  spaces: [
    { id: 'surya', name: 'surya', gitDetected: true },
    { id: 'comet', name: 'comet', gitDetected: true },
    { id: 'notes', name: 'notes', gitDetected: false },
  ],
  devices: [
    { id: 'dtry', name: 'dtry', online: true, local: true },
    { id: 'devbox', name: 'devbox', online: true, local: false },
    { id: 'mini', name: 'mini', online: false, local: false },
  ],
  favorites: ['claude-code:claude-opus-5', 'opencode:kimi-k3'],
  selectedHarness: 'claude-code',
  selectedModelId: 'claude-opus-5',
  selectedSpaceId: 'surya',
  selectedDeviceId: 'dtry',
  branch: 'main',
  checkout: 'local',
  yolo: 'off',
  query: '',
  switchError: null,
  refTotal: 312,
};

/// Nothing runnable resolved and nothing loaded: the no-agents card, the empty
/// ref list, and a device with no projects on it.
export const emptyPickers: PickersFixture = {
  ...seededPickers,
  harnesses: [],
  models: { 'claude-code': [], 'opencode': [], 'cursor': [] },
  refs: [],
  spaces: [],
  devices: [],
  favorites: [],
  selectedHarness: null,
  selectedModelId: null,
  selectedSpaceId: null,
  selectedDeviceId: null,
  branch: null,
  refTotal: 0,
};
