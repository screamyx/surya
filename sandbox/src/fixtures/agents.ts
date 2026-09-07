// Fixture props only. The native engine owns the watches, the tree walk and
// the roll-up; these are the rows they produce.
// Rust: app/crates/ui/src/inbox/mod.rs demo::agent_states() and demo::chats(),
// already grouped by app/crates/ui/src/inbox/model.rs agent_sections().
import type { AgentSection } from '../screens/Agents';

/// The native --inbox-demo rows, exactly. All four land under one heading
/// because every top-level row's rolled-up state wants the user.
export const demoSections: readonly AgentSection[] = [
  {
    group: 'waiting-for-you',
    rows: [
      {
        id: 'chat-migrate', chatId: 'chat-migrate', name: 'Add the follow-up column', depth: 0,
        state: 'needs-you-permission', rolledUp: 'needs-you-permission',
        needsYouChildren: 0, rolledUpOnly: false,
      },
      // The roll-up case: the parent is working, its child is blocked.
      {
        id: 'chat-fold', chatId: 'chat-fold', name: 'Rewrite the fold', depth: 0,
        state: 'working', rolledUp: 'needs-you-question',
        needsYouChildren: 1, rolledUpOnly: true,
      },
      {
        id: 'chat-fold:tool-9', chatId: 'chat-fold', name: 'scan the fold call sites', depth: 1,
        state: 'needs-you-question', rolledUp: 'needs-you-question',
        needsYouChildren: 0, rolledUpOnly: false,
      },
      {
        id: 'chat-import', chatId: 'chat-import', name: "Import last season's rows", depth: 0,
        state: 'stopped', rolledUp: 'stopped',
        needsYouChildren: 0, rolledUpOnly: false,
      },
    ],
  },
];

/// The demo rows plus one run per remaining heading, so a designer sees all
/// four groups, every state tone and the indent at depth 0 through 3.
/// Made-up work in a made-up repo, per decision 18.
export const seededSections: readonly AgentSection[] = [
  demoSections[0],
  {
    group: 'running',
    rows: [
      {
        id: 'chat-index', chatId: 'chat-index', name: 'Reindex the parts catalogue', depth: 0,
        state: 'working', rolledUp: 'working', needsYouChildren: 0, rolledUpOnly: false,
      },
      {
        id: 'chat-index:tool-2', chatId: 'chat-index', name: 'read the vendor feed', depth: 1,
        state: 'working', rolledUp: 'working', needsYouChildren: 0, rolledUpOnly: false,
      },
      {
        id: 'chat-index:tool-2:tool-4', chatId: 'chat-index', name: 'match the supplier codes', depth: 2,
        state: 'done', rolledUp: 'done', needsYouChildren: 0, rolledUpOnly: false,
      },
      {
        id: 'chat-index:tool-2:tool-4:tool-7', chatId: 'chat-index', name: 'write the merged rows into the staging table', depth: 3,
        state: 'working', rolledUp: 'working', needsYouChildren: 0, rolledUpOnly: false,
      },
    ],
  },
  {
    group: 'done',
    rows: [
      {
        id: 'chat-branch', chatId: 'chat-branch', name: 'Split the branch picker', depth: 0,
        state: 'done', rolledUp: 'done', needsYouChildren: 0, rolledUpOnly: false,
      },
    ],
  },
  {
    group: 'idle',
    rows: [
      {
        id: 'chat-notes', chatId: 'chat-notes', name: 'Draft the release notes', depth: 0,
        state: 'idle', rolledUp: 'idle', needsYouChildren: 0, rolledUpOnly: false,
      },
    ],
  },
];

/// Nothing running anywhere: the rail draws its one-sentence quiet state.
export const emptySections: readonly AgentSection[] = [];
