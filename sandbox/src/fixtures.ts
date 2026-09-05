// Fixture props only. The native engine continues to own subscriptions and routing.
import type { InboxRow } from './screens/NeedsYou';
export const seededRows: readonly InboxRow[] = [
  { id: 'question-suites', chatId: 'wire-tasks', prompt: 'Which suites should gate the merge?' },
  { id: 'question-sync', chatId: 'wire-tasks', prompt: 'Which sync strategy should the rewrite use?' },
];
export type Fixture = 'seeded' | 'empty';
export const emptyChats = [
  { title: 'Dock Row Tone Feature Work', project: 'luvus @ pc-ajim', age: '4h', branch: 'dock-row-tone' },
  { title: 'Optional Tone And Spans', project: 'luvus @ pc-ajim', age: '14h', branch: 'dock-row-tone' },
  { title: 'Display All Leads Table', project: '@ pc-ajim', age: '15h', branch: '' },
  { title: '**Local Environment Configuration', project: 'Downloads @ pc-ajim', age: '19h', branch: '' },
];
