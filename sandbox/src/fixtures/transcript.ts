// Fixture props for the chat transcript. The native engine keeps its doc
// watch, its parser, its highlighter and its clock; these are the shapes
// rows_for_entry hands the render, already derived.
import type { TranscriptRow } from '../screens/Transcript';
import type { WorkingTrailer } from '../screens/transcript/working';
import type { PermissionAsk } from '../screens/transcript/permission_card';

/// Rust: `tool_group_summary` - "Ran 3 commands · edited 2 files", with the
/// UI-synthesized thought chips named on the same line.
export const seededRows: readonly TranscriptRow[] = [
  {
    id: 'm1#u0',
    entryId: 'm1',
    partKey: 'm1#u',
    turnStart: true,
    timestamp: 'Sep 7, 9:14 PM',
    copyText: 'Why does the folder filter only search one page?',
    stopped: false,
    kind: {
      type: 'user',
      spans: [
        { text: 'Why does ', mention: false },
        { text: '@picker.rs', mention: true },
        { text: ' only search one page of the folder?', mention: false },
      ],
      attachments: [],
      badges: [{ label: 'Diff', count: 3 }],
      pending: false,
    },
  },
  {
    id: 'm2#t0',
    entryId: 'm2',
    partKey: 'm2#t',
    turnStart: true,
    stopped: false,
    kind: {
      type: 'toolGroup',
      autoOpen: false,
      summary: 'Thought · Ran 2 commands · read 1 file',
      tools: [
        {
          id: 'm2#t0-0',
          glyph: 'chat-round-line',
          label: 'Thought process',
          detail: '',
          isThought: true,
          body: {
            type: 'thought',
            lines: [
              'The filter reads the page the picker has already loaded.',
              'That is why a folder deeper than one page never matches.',
              '',
              'The fix walks the whole folder before it filters.',
            ],
            truncatedBy: 0,
          },
        },
        {
          id: 'm2#t0-1',
          glyph: 'command',
          label: 'Command',
          detail: 'rg -n "fn filter" app/crates/ui/src/picker.rs',
          invocation: {
            type: 'output',
            lines: ['rg -n "fn filter" app/crates/ui/src/picker.rs'],
            truncatedBy: 0,
          },
          body: {
            type: 'output',
            lines: [
              '412:    fn filter(&self, query: &str) -> Vec<Entry> {',
              '447:    fn filter_page(&self, page: &Page) -> Vec<Entry> {',
            ],
            truncatedBy: 0,
          },
        },
        {
          id: 'm2#t0-2',
          glyph: 'document',
          label: 'Read',
          detail: 'app/crates/ui/src/picker.rs',
          body: {
            type: 'output',
            lines: [
              'pub struct FolderPicker {',
              '    page: Page,',
              '    query: String,',
              '}',
            ],
            truncatedBy: 118,
          },
        },
        {
          id: 'm2#t0-3',
          glyph: 'pen',
          label: 'Edit',
          detail: 'app/crates/ui/src/picker.rs',
          isError: true,
          body: { type: 'stats', stats: [{ path: 'app/crates/ui/src/picker.rs', additions: 24, deletions: 9 }] },
        },
      ],
    },
  },
  {
    id: 'm2#p0b0',
    entryId: 'm2',
    partKey: 'm2#p0',
    turnStart: false,
    stopped: false,
    kind: {
      type: 'markdown',
      block: { type: 'heading', level: 2, runs: [{ text: 'What the filter actually reads' }] },
    },
  },
  {
    id: 'm2#p0b1',
    entryId: 'm2',
    partKey: 'm2#p0',
    turnStart: false,
    stopped: false,
    kind: {
      type: 'markdown',
      block: {
        type: 'paragraph',
        runs: [
          { text: 'The filter runs over ' },
          { text: 'self.page', code: true },
          { text: ', which holds only the entries the picker has already loaded. A folder that spans more than one page never matches past the first, so ' },
          { text: 'the query looks broken', bold: true },
          { text: ' when it is really looking at the wrong set.' },
        ],
      },
    },
  },
  {
    id: 'm2#p0b2',
    entryId: 'm2',
    partKey: 'm2#p0',
    turnStart: false,
    stopped: false,
    kind: {
      type: 'markdown',
      block: {
        type: 'list',
        items: [
          [{ type: 'paragraph', runs: [{ text: 'Walk the folder before filtering, not after.' }] }],
          [{ type: 'paragraph', runs: [{ text: 'Keep the page cursor for paint, not for the query.' }] }],
          [{ type: 'paragraph', runs: [{ text: 'Cover it with a folder deeper than one page.' }] }],
        ],
      },
    },
  },
  {
    id: 'm2#p0b3',
    entryId: 'm2',
    partKey: 'm2#p0',
    turnStart: false,
    stopped: false,
    kind: {
      type: 'markdown',
      block: {
        type: 'code',
        language: 'rust',
        code: 'fn filter(&self, query: &str) -> Vec<Entry> {\n    self.walk_folder()\n        .filter(|entry| entry.name.contains(query))\n        .take(MAX_RESULTS)\n        .collect()\n}',
      },
    },
  },
  {
    id: 'm2#p0b4',
    entryId: 'm2',
    partKey: 'm2#p0',
    turnStart: false,
    stopped: false,
    kind: {
      type: 'markdown',
      block: {
        type: 'quote',
        children: [{
          type: 'paragraph',
          runs: [{ text: 'Walking the folder costs one extra read on open. Filtering the wrong set costs the feature.' }],
        }],
      },
    },
  },
  {
    id: 'm3#perm0',
    entryId: 'm3',
    partKey: 'm3#perm',
    turnStart: true,
    stopped: false,
    kind: {
      type: 'permissionChip',
      toolName: 'Command',
      command: 'cargo test -p surya-ui',
      resolved: true,
      allowed: true,
      always: true,
    },
  },
  {
    id: 'm3#q0',
    entryId: 'm3',
    partKey: 'm3#q',
    turnStart: false,
    stopped: false,
    kind: { type: 'inputChip', header: 'Which suites should gate the merge?', resolved: false },
  },
  {
    id: 'm3#err0',
    entryId: 'm3',
    partKey: 'm3#err',
    turnStart: false,
    stopped: false,
    kind: {
      type: 'errorChip',
      message: 'The agent exited with status 1 before the first turn: could not open the engine socket at /run/surya/engine.sock.',
    },
  },
  {
    id: 'm4#p0b0',
    entryId: 'm4',
    partKey: 'm4#p0',
    turnStart: true,
    timestamp: 'Sep 7, 9:21 PM',
    copyText: 'Counting the folder entries: 1. one 2. two 25. twe',
    stopped: true,
    retryPrompt: 'Count every entry in the folder',
    kind: {
      type: 'liveMarkdown',
      block: {
        type: 'paragraph',
        runs: [{ text: 'Counting the folder entries: 1. one 2. two 25. twe' }],
      },
    },
  },
];

/// The turn that is still running: the flavour word rotates every 7s and the
/// timer starts with the turn, not with the send.
export const workingNow: WorkingTrailer = {
  word: 'Untangling',
  elapsed: '1m 32s',
  sending: false,
  queued: false,
};

/// The card the composer hosts while a tool waits on an answer.
export const pendingPermission: PermissionAsk = {
  requestId: 'perm-1',
  toolName: 'Command',
  command: 'git push origin sandbox/port-in-transcript',
};

/// The send that has not landed: an attachment still crossing the relay above
/// a prompt the doc has not confirmed.
export const sendingRows: readonly TranscriptRow[] = [
  {
    id: 'p1#u0',
    entryId: 'p1',
    partKey: 'p1#u',
    turnStart: true,
    stopped: false,
    kind: {
      type: 'user',
      spans: [{ text: 'Here is the frame that clips the last tool call.', mention: false }],
      attachments: [
        { name: 'clipped-chips.png', state: 'loaded', sending: true, percent: 62 },
        { name: 'reference.png', state: 'loading' },
        { name: 'gone.png', state: 'error' },
      ],
      badges: [],
      pending: true,
    },
  },
];
