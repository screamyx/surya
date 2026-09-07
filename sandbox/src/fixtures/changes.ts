// Fixture props only. The native engine continues to own the diff watch,
// the patch parse and the syntax highlight cache.
import type { FileDiff, ParsedDiff } from '../screens/changes/model';
import type { ChangeRequestSummary } from '../screens/changes/change_requests';

/// Modified file: two hunks, matching the parser test patch in changes.rs.
const mainRs: FileDiff = {
  path: 'app/crates/ui/src/main.rs',
  oldPath: null,
  status: 'modified',
  binary: false,
  notices: [],
  additions: 3,
  deletions: 2,
  maxLine: 12,
  hunks: [
    {
      header: '@@ -1,4 +1,5 @@ fn main',
      lines: [
        { kind: 'context', oldNo: 1, newNo: 1, text: 'fn main() {' },
        { kind: 'del', oldNo: 2, newNo: null, text: '    println!("old");' },
        { kind: 'add', oldNo: null, newNo: 2, text: '    println!("new");' },
        { kind: 'add', oldNo: null, newNo: 3, text: '    let x = 1;' },
        { kind: 'context', oldNo: 3, newNo: 4, text: '}' },
      ],
    },
    {
      header: '@@ -10,2 +11,2 @@',
      lines: [
        { kind: 'context', oldNo: 10, newNo: 11, text: '// tail' },
        { kind: 'del', oldNo: 11, newNo: null, text: 'old_line' },
        { kind: 'add', oldNo: null, newNo: 12, text: 'new_line' },
      ],
    },
  ],
};

/// Added file, ending without a trailing newline: exercises the New file
/// notice and the meta row that spans both split columns.
const addedTxt: FileDiff = {
  path: 'added.txt',
  oldPath: null,
  status: 'added',
  binary: false,
  notices: [],
  additions: 2,
  deletions: 0,
  maxLine: 2,
  hunks: [
    {
      header: '@@ -0,0 +1,2 @@',
      lines: [
        { kind: 'add', oldNo: null, newNo: 1, text: 'first' },
        { kind: 'add', oldNo: null, newNo: 2, text: 'second' },
        { kind: 'meta', oldNo: null, newNo: null, text: '\\ No newline at end of file' },
      ],
    },
  ],
};

const goneTxt: FileDiff = {
  path: 'gone.txt',
  oldPath: null,
  status: 'deleted',
  binary: false,
  notices: [],
  additions: 0,
  deletions: 1,
  maxLine: 1,
  hunks: [
    { header: '@@ -1,1 +0,0 @@', lines: [{ kind: 'del', oldNo: 1, newNo: null, text: 'bye' }] },
  ],
};

/// Binary file: no hunks, so the body is its two notices and nothing else.
const imgPng: FileDiff = {
  path: 'docs/img.png',
  oldPath: null,
  status: 'added',
  binary: true,
  notices: [],
  additions: 0,
  deletions: 0,
  maxLine: 0,
  hunks: [],
};

/// Renamed file with a four-digit line number: widens the gutter step.
const renamed: FileDiff = {
  path: 'app/crates/ui/src/new_name.rs',
  oldPath: 'app/crates/ui/src/old_name.rs',
  status: 'renamed',
  binary: false,
  notices: ['Diff truncated — showing first 2 of 1180 lines'],
  additions: 1,
  deletions: 1,
  maxLine: 1204,
  hunks: [
    {
      header: '@@ -1198,4 +1201,4 @@ impl Changes',
      lines: [
        { kind: 'context', oldNo: 1198, newNo: 1201, text: '    fn render_header_strip(&self, theme: &Theme) -> Option<AnyElement> {' },
        { kind: 'del', oldNo: 1199, newNo: null, text: '        let parsed = self.parsed.as_ref()?;' },
        { kind: 'add', oldNo: null, newNo: 1202, text: '        let parsed = self.parsed.as_ref().or(self.pending.as_ref())?;' },
        { kind: 'context', oldNo: 1200, newNo: 1203, text: '        Some(' },
      ],
    },
  ],
};

export const seededDiff: ParsedDiff = {
  truncated: true,
  additions: 6,
  deletions: 4,
  files: [mainRs, addedTxt, goneTxt, imgPng, renamed],
};

/// Rust: DiffPhase::Clean. A diff arrived and it was empty.
export const emptyDiff: ParsedDiff = { truncated: false, additions: 0, deletions: 0, files: [] };

export const seededChangeRequest: ChangeRequestSummary = {
  number: 186,
  state: 'open',
  title: 'fix(picker): the folder filter searches the whole folder, not one page',
  url: 'https://github.com/orchard/surya/pull/186',
};
