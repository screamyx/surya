// Fixture props for the Tasks board. The rows are the native demo seed in
// app/scripts/tasks-demo-shot.sh, which is what `surya --tasks-demo`
// (app/crates/ui/src/tasks/demo.rs) shows. The native engine keeps the watch,
// the mutates and the rank arithmetic.
import type { BoardTask } from '../screens/tasks/model';

export const tasksSpaceName = 'project-jag';

/// Board order: the seed sets no rank, so sort_tasks falls through to
/// created_at and the cards sit in creation order.
export const seededTasks: readonly BoardTask[] = [
  {
    id: 't-5', spaceId: 'space-demo', status: 'queued', rank: 0, createdAt: '09:41',
    title: 'Media tab scrolls sideways after Library upload (#553)',
    links: ['https://github.com/orchard/project-jag/issues/553'],
  },
  {
    id: 't-1', spaceId: 'space-demo', status: 'running', rank: 0, createdAt: '09:42',
    title: 'Add follow-up fields to leads', owner: 'raven',
    notes: 'migration + resource, PR open', links: [],
  },
  {
    id: 't-2', spaceId: 'space-demo', status: 'running', rank: 0, createdAt: '09:43',
    title: 'Reminder job the morning after a lead', owner: 'kite', links: [],
  },
  {
    id: 't-3', spaceId: 'space-demo', status: 'blocked', rank: 0, createdAt: '09:44',
    title: 'Show next follow-up on the lead card in the PWA',
    notes: 'waits on t-1 and t-2', links: [],
  },
  {
    id: 't-4', spaceId: 'space-demo', status: 'done', rank: 0, createdAt: '09:45',
    title: 'Tile jumps when a photo finishes uploading', owner: 'heron',
    links: ['https://github.com/orchard/project-jag/pull/591'],
  },
  {
    id: 't-6', spaceId: 'space-demo', status: 'queued', rank: 0, createdAt: '09:46',
    title: 'Publish the 12 new reels to the catalog', owner: 'swift', links: [],
  },
];

export const emptyTasks: readonly BoardTask[] = [];

/// The last RPC failure the board shows under the header.
export const tasksError = 'task board unavailable: connection refused';

/// CardGhost is a drag lifecycle the sandbox cannot carry; this prop paints
/// the ghost so a designer can see it.
export const tasksGhostTitle = 'Add follow-up fields to leads';
