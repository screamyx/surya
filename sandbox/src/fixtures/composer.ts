// Fixture props for the composer screen.
// Rust model: app/crates/ui/src/composer.rs (Composer, ComposerInput),
// pickers.rs (Pickers), attachments.rs (StagedAttachment, PreviewImage),
// settings/composer.rs (ComposerDefaults: remembered harness, model, favorites),
// key_chips.rs (the chord labels), permission_options.rs (the answer rows).

// One painted run of the input surface. GPUI paints its own editor
// (ComposerTextElement::prepaint): text runs, mention chip quads, selection
// quads, the caret quad and the shaped ghost suffix. The sandbox paints the
// same five things from fixture data; it does not accept typing.
export type InputSegment =
  | { kind: 'text'; text: string }
  | { kind: 'mention'; text: string; path: string }
  | { kind: 'selected'; text: string }
  | { kind: 'caret' }
  | { kind: 'ghost'; text: string };

export type InputLine = { id: string; segments: readonly InputSegment[] };

export type StagedAttachment = { id: string; name: string; extension: string };

export type SendMode = 'send' | 'steer' | 'stop';
export type YoloState = 'off' | 'on' | 'always';
export type PickerKind = 'model' | 'device' | 'project' | 'checkout' | 'branch';
export type Completion = 'none' | 'mention' | 'slash';

export type HarnessTab = { id: string; name: string; disabled: boolean };
export type ModelRow = {
  id: string;
  harnessId: string;
  harnessName: string;
  label: string;
  attribution: string;
  favorite: boolean;
};

export type MentionResult = { path: string; name: string; directory: string; isDir: boolean };
export type SlashCommand = { name: string; description: string };

export type PermissionRequest = { requestId: string; toolName: string; command: string };
export type WizardQuestion = {
  header: string;
  question: string;
  multiSelect: boolean;
  options: readonly string[];
};
export type WizardModel = { page: number; questions: readonly WizardQuestion[] };

export type ComposerModel = {
  // ComposerInput: what the text surface paints.
  lines: readonly InputLine[];
  isPlaceholder: boolean;
  // Composer::render.
  expanded: boolean;
  newChat: boolean;
  sendMode: SendMode;
  sendBlocked: boolean;
  attachments: readonly StagedAttachment[];
  comments: number;
  failure: string | null;
  failureOffline: boolean;
  queueNotice: { text: string; offline: boolean } | null;
  steerQueues: boolean;
  completion: Completion;
  completionLoading: boolean;
  completionError: string | null;
  mentionResults: readonly MentionResult[];
  slashCommands: readonly SlashCommand[];
  permission: PermissionRequest | null;
  wizard: WizardModel | null;
  // Pickers::render and its footer/target rows.
  yolo: YoloState;
  modelLabel: string;
  modelTraits: string | null;
  traitsCustomized: boolean;
  noAgents: boolean;
  harnessTabs: readonly HarnessTab[];
  models: readonly ModelRow[];
  selectedModelId: string;
  device: string;
  deviceOffline: boolean;
  project: string;
  git: boolean;
  checkout: string;
  branch: string;
};

const HARNESS_TABS: readonly HarnessTab[] = [
  { id: 'claude-code', name: 'Claude Code', disabled: false },
  { id: 'opencode', name: 'opencode', disabled: false },
  { id: 'cursor', name: 'Cursor', disabled: true },
];

const MODELS: readonly ModelRow[] = [
  { id: 'opus-5', harnessId: 'claude-code', harnessName: 'Claude Code', label: 'Opus 5', attribution: '1M context', favorite: true },
  { id: 'fable-5-1', harnessId: 'claude-code', harnessName: 'Claude Code', label: 'Fable 5.1', attribution: '', favorite: false },
  { id: 'sonnet-5', harnessId: 'claude-code', harnessName: 'Claude Code', label: 'Sonnet 5', attribution: '', favorite: false },
  { id: 'haiku-4-5', harnessId: 'claude-code', harnessName: 'Claude Code', label: 'Haiku 4.5', attribution: '', favorite: false },
  { id: 'glm-5-2-a', harnessId: 'opencode', harnessName: 'opencode', label: 'GLM-5.2', attribution: 'Anthropic', favorite: false },
  { id: 'glm-5-2-b', harnessId: 'opencode', harnessName: 'opencode', label: 'GLM-5.2', attribution: 'Baseten', favorite: true },
];

const MENTION_RESULTS: readonly MentionResult[] = [
  { path: 'app/crates/ui/src/composer.rs', name: 'composer.rs', directory: 'app/crates/ui/src', isDir: false },
  { path: 'app/crates/ui/src/composer', name: 'composer', directory: 'app/crates/ui/src', isDir: true },
  { path: 'app/crates/ui/src/attachments.rs', name: 'attachments.rs', directory: 'app/crates/ui/src', isDir: false },
  { path: 'app/crates/ui/src/key_chips.rs', name: 'key_chips.rs', directory: 'app/crates/ui/src', isDir: false },
];

const SLASH_COMMANDS: readonly SlashCommand[] = [
  { name: 'compact', description: 'Summarise the conversation so far' },
  { name: 'review', description: 'Review the current diff' },
  { name: 'clear', description: 'Start a fresh context' },
  { name: 'model', description: 'Switch the model for this chat · <name>' },
];

const BASE: ComposerModel = {
  lines: [],
  isPlaceholder: false,
  expanded: true,
  newChat: false,
  sendMode: 'send',
  sendBlocked: false,
  attachments: [],
  comments: 0,
  failure: null,
  failureOffline: false,
  queueNotice: null,
  steerQueues: false,
  completion: 'none',
  completionLoading: false,
  completionError: null,
  mentionResults: MENTION_RESULTS,
  slashCommands: SLASH_COMMANDS,
  permission: null,
  wizard: null,
  yolo: 'off',
  modelLabel: 'Opus 5',
  modelTraits: 'High',
  traitsCustomized: false,
  noAgents: false,
  harnessTabs: HARNESS_TABS,
  models: MODELS,
  selectedModelId: 'opus-5',
  device: 'dtry',
  deviceOffline: false,
  project: 'surya',
  git: true,
  checkout: 'Worktree',
  branch: 'sandbox/port-in',
};

const TYPED: readonly InputLine[] = [
  {
    id: 'line-1',
    segments: [
      { kind: 'text', text: 'Trace the flip in ' },
      { kind: 'mention', text: '@composer.rs', path: 'app/crates/ui/src/composer.rs' },
      { kind: 'text', text: ' and tell me which' },
    ],
  },
  {
    id: 'line-2',
    segments: [
      { kind: 'text', text: 'measurement drives the collapse.' },
      { kind: 'caret' },
    ],
  },
];

const SELECTED: readonly InputLine[] = [
  TYPED[0],
  {
    id: 'line-2',
    segments: [
      { kind: 'text', text: 'measurement ' },
      { kind: 'selected', text: 'drives the collapse' },
      { kind: 'text', text: '.' },
    ],
  },
];

const PLACEHOLDER: readonly InputLine[] = [
  { id: 'line-1', segments: [{ kind: 'caret' }, { kind: 'text', text: 'Do anything...' }] },
];

const GHOSTED: readonly InputLine[] = [
  {
    id: 'line-1',
    segments: [
      { kind: 'text', text: '/rev' },
      { kind: 'caret' },
      { kind: 'ghost', text: 'iew' },
    ],
  },
];

export const composerSeeded: ComposerModel = {
  ...BASE,
  lines: TYPED,
  sendMode: 'steer',
  attachments: [
    { id: 'att-1', name: 'flip-morph.png', extension: 'png' },
    { id: 'att-2', name: 'compact-pill.png', extension: 'png' },
  ],
  comments: 2,
  modelTraits: 'High · 1M',
  traitsCustomized: true,
};

export const composerEmpty: ComposerModel = {
  ...BASE,
  lines: PLACEHOLDER,
  isPlaceholder: true,
  newChat: true,
  sendBlocked: true,
  project: 'No project',
  git: false,
  modelTraits: null,
};

export const composerVariants: Readonly<Record<string, ComposerModel>> = {
  seeded: composerSeeded,
  empty: composerEmpty,
  selection: { ...BASE, lines: SELECTED, sendMode: 'steer' },
  compact: { ...BASE, expanded: false, lines: [{ id: 'line-1', segments: [{ kind: 'text', text: 'ship it' }, { kind: 'caret' }] }] },
  stop: { ...BASE, lines: PLACEHOLDER, isPlaceholder: true, sendMode: 'stop' },
  mention: { ...BASE, lines: TYPED, completion: 'mention' },
  slash: { ...BASE, lines: GHOSTED, completion: 'slash' },
  failure: { ...BASE, lines: TYPED, failure: 'Engine not connected', failureOffline: true, queueNotice: { text: 'Offline - messages will send when you are back online.', offline: true } },
  steer: { ...BASE, lines: TYPED, sendMode: 'steer', steerQueues: true, yolo: 'always' },
  permission: { ...BASE, permission: { requestId: 'req-1', toolName: 'Bash', command: 'cargo test -p surya-ui' } },
  wizard: {
    ...BASE,
    lines: [{ id: 'line-1', segments: [{ kind: 'caret' }, { kind: 'text', text: 'Type your own answer, or pick an option above' }] }],
    isPlaceholder: true,
    wizard: {
      page: 0,
      questions: [
        { header: 'Suites', question: 'Which suites should gate the merge?', multiSelect: true, options: ['surya-ui only', 'surya-ui and surya-a2ui', 'The whole workspace'] },
        { header: 'Sync', question: 'Which sync strategy should the rewrite use?', multiSelect: false, options: ['Watch the engine', 'Poll on focus'] },
      ],
    },
  },
  noAgents: { ...BASE, lines: PLACEHOLDER, isPlaceholder: true, noAgents: true, modelLabel: 'No agents available', modelTraits: null, sendBlocked: true },
};
