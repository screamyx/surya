// Fixture props for the git history screen. The native engine continues to own
// the LIST_GIT_HISTORY call, paging, the clipboard and the fetch task.
// The graph rows below are the real output of history.rs::layout_graph for these
// eight commits with a7c1f30 as HEAD, not a hand-drawn approximation.
import type { GitHistoryProps, HistoryCommit, HistoryGraph } from '../screens/History';

/// history.rs::ref_area_width evaluated for the 1440x900 reference client with
/// the shell rail open: an 890 px commit column yields a 397 px ref area.
/// container_query() supplies this natively, on every resize.
export const referenceRefAreaWidth = 397;

export const seededCommits: readonly HistoryCommit[] = [
  {
    sha: 'a7c1f3049e2b6d8a5c1477fe0b93a2d611c8ee40',
    parentShas: ['4261fa7', '6d461b9'],
    subject: 'Merge pull request #186 from screamyx/fix-folder-filter',
    authorName: 'screamyx',
    authorEmail: 'screamyx@example.com',
    authoredAt: '2026-09-07T18:41:02+08:00',
    refs: [
      { kind: 'branch', label: 'main' },
      { kind: 'remote', label: 'origin/main' },
      { kind: 'remote', label: 'upstream/main' },
      { kind: 'tag', label: 'v0.1.53' },
      { kind: 'branch', label: 'release/2026-09' },
    ],
  },
  {
    sha: '4261fa76b0d13e8f5a2c9047bb6e1d3a8c05f912',
    parentShas: ['adaf897'],
    subject: 'fix(picker): the folder filter searches the whole folder, not one page',
    authorName: 'screamyx',
    authorEmail: 'screamyx@example.com',
    authoredAt: '2026-09-07T16:12:44+08:00',
    refs: [],
  },
  {
    sha: '6d461b9a3f8c07e25d1b94a6cc03f8e17b2d5a64',
    parentShas: ['adaf897'],
    subject: 'fix(ui): the folder picker names Ctrl on Windows, not the Command glyph',
    authorName: 'Dana Okonkwo',
    authorEmail: 'dana@example.com',
    authoredAt: '2026-09-06T22:03:19+08:00',
    refs: [{ kind: 'remote', label: 'origin/fix-folder-picker' }],
  },
  {
    sha: 'adaf897b1c6045ea92f38b7d5106ce43a8f2b0d7',
    parentShas: ['e860f77'],
    subject: 'fix(ui): fold text_dim, a text tier that stopped being real',
    authorName: 'screamyx',
    authorEmail: 'screamyx@example.com',
    authoredAt: '2026-09-06T11:58:07+08:00',
    refs: [],
  },
  {
    sha: 'e860f7742a95b03cd6178ef4b029a5c31d70668b',
    parentShas: ['70d66f1'],
    subject: 'fix(chat): a stopped run can be retried',
    authorName: 'Priya Raghunathan',
    authorEmail: 'priya@example.com',
    authoredAt: '2026-09-05T09:24:51+08:00',
    refs: [{ kind: 'tag', label: 'v0.1.52' }],
  },
  {
    sha: '70d66f1e5b2843ac09d7f61b3e05c928a4d1f7b3',
    parentShas: ['2ebc1ad'],
    subject: 'fix(ui): the Agents rail entry reveals the tree it names',
    authorName: 'screamyx',
    authorEmail: 'screamyx@example.com',
    authoredAt: '2026-09-04T20:47:33+08:00',
    refs: [],
  },
  {
    sha: '2ebc1ad5093f7e416b8dc25a07f3491ce6b8204a',
    parentShas: ['91b4c05'],
    subject: 'chore: neutral machine names in comments, docs and fixtures',
    authorName: 'Tomasz Wieczorek',
    authorEmail: 'tomasz@example.com',
    authoredAt: '2026-09-03T14:05:12+08:00',
    refs: [],
  },
  {
    sha: '91b4c05d7e3a0128fb64c9d5230ae817f4b6039c',
    parentShas: [],
    subject: '',
    authorName: '',
    authorEmail: '',
    authoredAt: '2026-08-29T08:30:00+08:00',
    refs: [],
  },
];

export const seededGraph: HistoryGraph = {
  maxLaneCount: 2,
  rows: [
    { sha: 'a7c1f3049e2b6d8a5c1477fe0b93a2d611c8ee40', nodeLane: 0, nodeColorId: 0, isHead: true, segments: [{ fromLane: 0, toLane: 0, colorId: 0, shape: 'outgoing' }, { fromLane: 0, toLane: 1, colorId: 1, shape: 'outgoing' }] },
    { sha: '4261fa76b0d13e8f5a2c9047bb6e1d3a8c05f912', nodeLane: 0, nodeColorId: 0, isHead: false, segments: [{ fromLane: 1, toLane: 1, colorId: 1, shape: 'through' }, { fromLane: 0, toLane: 0, colorId: 0, shape: 'incoming' }, { fromLane: 0, toLane: 0, colorId: 0, shape: 'outgoing' }] },
    { sha: '6d461b9a3f8c07e25d1b94a6cc03f8e17b2d5a64', nodeLane: 1, nodeColorId: 1, isHead: false, segments: [{ fromLane: 0, toLane: 0, colorId: 0, shape: 'through' }, { fromLane: 1, toLane: 1, colorId: 1, shape: 'incoming' }, { fromLane: 1, toLane: 1, colorId: 1, shape: 'outgoing' }] },
    { sha: 'adaf897b1c6045ea92f38b7d5106ce43a8f2b0d7', nodeLane: 0, nodeColorId: 0, isHead: false, segments: [{ fromLane: 0, toLane: 0, colorId: 0, shape: 'incoming' }, { fromLane: 1, toLane: 0, colorId: 1, shape: 'incoming' }, { fromLane: 0, toLane: 0, colorId: 0, shape: 'outgoing' }] },
    { sha: 'e860f7742a95b03cd6178ef4b029a5c31d70668b', nodeLane: 0, nodeColorId: 0, isHead: false, segments: [{ fromLane: 0, toLane: 0, colorId: 0, shape: 'incoming' }, { fromLane: 0, toLane: 0, colorId: 0, shape: 'outgoing' }] },
    { sha: '70d66f1e5b2843ac09d7f61b3e05c928a4d1f7b3', nodeLane: 0, nodeColorId: 0, isHead: false, segments: [{ fromLane: 0, toLane: 0, colorId: 0, shape: 'incoming' }, { fromLane: 0, toLane: 0, colorId: 0, shape: 'outgoing' }] },
    { sha: '2ebc1ad5093f7e416b8dc25a07f3491ce6b8204a', nodeLane: 0, nodeColorId: 0, isHead: false, segments: [{ fromLane: 0, toLane: 0, colorId: 0, shape: 'incoming' }, { fromLane: 0, toLane: 0, colorId: 0, shape: 'outgoing' }] },
    { sha: '91b4c05d7e3a0128fb64c9d5230ae817f4b6039c', nodeLane: 0, nodeColorId: 0, isHead: false, segments: [{ fromLane: 0, toLane: 0, colorId: 0, shape: 'incoming' }] },
  ],
};

export const emptyGraph: HistoryGraph = { rows: [], maxLaneCount: 0 };

const base = {
  hasRepository: true,
  commits: seededCommits,
  graph: seededGraph,
  hasMore: true,
  loading: false,
  error: null,
  fetchError: null,
  copiedSha: null,
  refAreaWidth: referenceRefAreaWidth,
};

export type HistoryFixture =
  'seeded' | 'copied' | 'empty' | 'loading' | 'paging' | 'error' | 'listError' | 'noRepository';

const withoutCommits = { ...base, commits: [], graph: emptyGraph, hasMore: false };

export const historyFixtures: Record<HistoryFixture, Omit<GitHistoryProps, 'onOpenCommit' | 'onCopySha' | 'onLoadOlder'>> = {
  seeded: base,
  copied: { ...base, copiedSha: seededCommits[3]?.sha ?? null },
  empty: withoutCommits,
  loading: { ...withoutCommits, loading: true },
  paging: { ...base, loading: true },
  error: { ...base, fetchError: 'could not reach origin: connection timed out' },
  listError: { ...base, error: 'git log failed: bad revision HEAD' },
  noRepository: { ...withoutCommits, hasRepository: false },
};

/// GitHistoryCount reads head_commit_count and the selected chat row's branch.
export const historyCount = 1284;
export const historyBranch = 'sandbox/port-in-sbx-history';
