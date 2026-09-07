// Rust: app/crates/ui/src/files/model.rs (Row, TreeModel::rows, move_selection)
// and files/tree.rs (FileTreeView::rows, the search branch).
// Pure derivation, no gpui and no JSX: the rows that fall out of the entries
// the engine has listed and the directories the owner has opened.

export type FileKind = 'dir' | 'file';

// One entry the engine listed. Mirrors surya_proto::files::FileEntry, minus
// `size`, which the tree never draws.
export type FileEntry = {
  path: string;
  kind: FileKind;
  // Git status letter: M, A, D, ? or empty.
  status: string;
  // A directory at the listing edge whose children were not sent.
  hasChildren: boolean;
};

// One drawable row. Mirrors files/model.rs Row.
export type TreeRow = {
  path: string;
  kind: FileKind;
  status: string;
  depth: number;
  // The name, or `a/b/c` for a folded chain.
  label: string;
  expanded: boolean;
  hasChildren: boolean;
};

function parentOf(path: string): string {
  const cut = path.lastIndexOf('/');
  return cut < 0 ? '' : path.slice(0, cut);
}

function byPath(entries: readonly FileEntry[]): FileEntry[] {
  return [...entries].sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
}

// files/model.rs TreeModel::rows, including haktui's fold rule: a directory
// whose only child is a directory folds into it, so `src/main/java/com` is one
// row and not four rows of nothing. Every fixture directory counts as listed,
// so `only_child_dir`'s `loaded` guard is always satisfied here.
export function treeRows(entries: readonly FileEntry[], expanded: readonly string[]): TreeRow[] {
  const sorted = byPath(entries);
  const children = new Map<string, FileEntry[]>();
  for (const entry of sorted) {
    const kids = children.get(parentOf(entry.path));
    if (kids) kids.push(entry);
    else children.set(parentOf(entry.path), [entry]);
  }
  const onlyChildDir = (dir: string): FileEntry | null => {
    const kids = children.get(dir);
    return kids !== undefined && kids.length === 1 && kids[0].kind === 'dir' ? kids[0] : null;
  };
  const foldedInto = new Map<string, string>();
  for (const entry of sorted) {
    if (entry.kind !== 'dir' || foldedInto.has(entry.path)) continue;
    const chain = [entry];
    let tail = entry;
    for (let next = onlyChildDir(tail.path); next !== null; next = onlyChildDir(tail.path)) {
      chain.push(next);
      tail = next;
    }
    if (chain.length > 1) {
      for (const folded of chain.slice(0, -1)) foldedInto.set(folded.path, tail.path);
    }
  }
  const isOpen = (dir: string): boolean => {
    const tail = foldedInto.get(dir);
    return expanded.includes(tail === undefined ? dir : tail);
  };
  const rows: TreeRow[] = [];
  for (const entry of sorted) {
    if (foldedInto.has(entry.path)) continue;
    const parent = parentOf(entry.path);
    let shown = true;
    let acc = '';
    for (const part of parent === '' ? [] : parent.split('/')) {
      acc = acc === '' ? part : `${acc}/${part}`;
      if (foldedInto.get(acc) === entry.path) continue;
      if (!isOpen(acc)) {
        shown = false;
        break;
      }
    }
    if (!shown) continue;
    let head = entry.path;
    while (head.includes('/') && foldedInto.get(parentOf(head)) === entry.path) head = parentOf(head);
    let depth = 0;
    let seg = '';
    for (const part of parent === '' ? [] : parent.split('/')) {
      seg = seg === '' ? part : `${seg}/${part}`;
      if (!foldedInto.has(seg)) depth += 1;
    }
    const headParent = parentOf(head);
    const label = head === entry.path
      ? entry.path.slice(entry.path.lastIndexOf('/') + 1)
      : entry.path.slice(headParent === '' ? 0 : headParent.length + 1);
    const kids = children.get(entry.path);
    rows.push({
      path: entry.path,
      kind: entry.kind,
      status: entry.status,
      depth,
      label,
      expanded: entry.kind === 'dir' && expanded.includes(entry.path),
      hasChildren: entry.kind === 'dir' && (entry.hasChildren || (kids !== undefined && kids.length > 0)),
    });
  }
  return rows;
}

// files/tree.rs FileTreeView::rows, the `Some(matches)` branch: a flat list of
// full paths at depth 0 with no status. The native matches come from a
// FilesSearch call; here they are a substring pass over the same fixture.
export function searchRows(entries: readonly FileEntry[], query: string): TreeRow[] {
  const needle = query.trim().toLowerCase();
  return byPath(entries)
    .filter(entry => entry.path.toLowerCase().includes(needle))
    .map(entry => ({
      path: entry.path,
      kind: entry.kind,
      status: '',
      depth: 0,
      label: entry.path,
      expanded: false,
      hasChildren: entry.kind === 'dir',
    }));
}

// files/model.rs TreeModel::move_selection: clamped, and an absent selection
// lands on the first row going down or the last going up.
export function moveSelection(rows: readonly TreeRow[], selected: string | null, delta: number): string | null {
  if (rows.length === 0) return null;
  const current = selected === null ? -1 : rows.findIndex(row => row.path === selected);
  if (current < 0) return delta >= 0 ? rows[0].path : rows[rows.length - 1].path;
  return rows[Math.min(Math.max(current + delta, 0), rows.length - 1)].path;
}

// files/tree.rs FileTreeView::reveal: expand every ancestor of `path`.
export function ancestorsOf(path: string): string[] {
  const out: string[] = [];
  let acc = '';
  for (const part of path.split('/')) {
    acc = acc === '' ? part : `${acc}/${part}`;
    if (acc !== path) out.push(acc);
  }
  return out;
}
