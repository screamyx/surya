// Rust: app/crates/ui/src/files/tree.rs (FileTreeView::render, render_row,
// on_key_down, activate, reveal, clear_filter).
import { useState } from 'react';
import type { FileEntry, TreeRow } from './model';
import { ancestorsOf, moveSelection, searchRows, treeRows } from './model';

export type FileTreeViewProps = {
  entries: readonly FileEntry[];
  // The directories the engine has listed and the owner has already opened.
  // TreeModel starts empty in Rust and fills as FilesTree answers arrive.
  expandedSeed: readonly string[];
  // TreeEvent::Open: a file row was activated.
  onOpen: (path: string) => void;
};

// files/tree.rs render_row, the (kind, expanded, has_children) match.
function chevronFor(row: TreeRow): string {
  if (row.kind !== 'dir') return ' ';
  if (row.expanded) return '▾';
  return row.hasChildren ? '▸' : '·';
}

// The native row indents with pl(px(8 + depth * 12)); the whitelist has no
// per-depth padding step, so the 8px sits on the row and the depth is a
// flex-none spacer. The row's gap-1 falls between the spacer and the chevron,
// so each width is the native indent less that 4px: the chevron lands on the
// native pixel. Exact to depth 5, clamped 4px short past it.
function indentFor(depth: number) {
  if (depth <= 0) return null;
  return <span aria-hidden="true" className={depth === 1 ? 'flex-none w-2'
    : depth === 2 ? 'flex-none w-5'
      : depth === 3 ? 'flex-none w-8'
        : depth === 4 ? 'flex-none w-11'
          : depth === 5 ? 'flex-none w-14'
            : 'flex-none w-16'} />;
}

// files/tree.rs FileTreeView::render_row. A render helper on the same view,
// not a second stateful component.
export function renderTreeRow(row: TreeRow, selected: boolean, onActivate: (path: string) => void) {
  return (
    <div key={row.path} role="treeitem" aria-selected={selected} data-row={row.path}
      aria-expanded={row.kind === 'dir' ? row.expanded : undefined}
      onMouseDown={() => onActivate(row.path)}
      className={selected
        ? (row.status === '?'
          ? 'flex items-center gap-1 h-6 pl-2 pr-2 text-ui-12 text-text-muted bg-element-active cursor-pointer hover:bg-element-hover'
          : 'flex items-center gap-1 h-6 pl-2 pr-2 text-ui-12 text-text bg-element-active cursor-pointer hover:bg-element-hover')
        : (row.status === '?'
          ? 'flex items-center gap-1 h-6 pl-2 pr-2 text-ui-12 text-text-muted cursor-pointer hover:bg-element-hover'
          : 'flex items-center gap-1 h-6 pl-2 pr-2 text-ui-12 text-text cursor-pointer hover:bg-element-hover')}>
      {indentFor(row.depth)}
      <span aria-hidden="true" className="flex-none w-2.5 text-text-faint">{chevronFor(row)}</span>
      <span className="flex-1 min-w-0 truncate whitespace-nowrap">{row.label}</span>
      {row.status === '' ? null : (
        <span aria-label={`status ${row.status}`} className={row.status === 'M' ? 'text-warning'
          : row.status === 'A' ? 'text-success'
            : row.status === 'D' ? 'text-danger'
              : row.status === '?' ? 'text-text-faint'
                : 'text-text-muted'}>{row.status}</span>
      )}
    </div>
  );
}

export function FileTreeView({ entries, expandedSeed, onOpen }: FileTreeViewProps) {
  const [expanded, setExpanded] = useState<readonly string[]>(expandedSeed);
  const [selected, setSelected] = useState<string | null>(null);
  const [filter, setFilter] = useState('');
  const [filterFocused, setFilterFocused] = useState(false);
  const filtering = filter.trim() !== '';
  const rows = filtering ? searchRows(entries, filter) : treeRows(entries, expanded);

  // files/tree.rs FileTreeView::activate.
  function activate(path: string) {
    const row = rows.find(candidate => candidate.path === path);
    setSelected(path);
    if (row === undefined || row.kind !== 'dir') {
      onOpen(path);
      return;
    }
    if (filtering) {
      // Jump out of the filter to the directory itself.
      setFilter('');
      setExpanded([...expanded, ...ancestorsOf(path).filter(dir => !expanded.includes(dir))]);
      return;
    }
    setExpanded(expanded.includes(path) ? expanded.filter(dir => dir !== path) : [...expanded, path]);
  }

  // files/tree.rs FileTreeView::on_key_down.
  function onKeyDown(key: string) {
    if (key === 'ArrowUp' || key === 'ArrowDown') {
      setSelected(moveSelection(rows, selected, key === 'ArrowUp' ? -1 : 1));
      return;
    }
    if (key === 'Escape') {
      setFilter('');
      return;
    }
    if (filterFocused) return;
    if (key === 'Enter') {
      if (selected !== null) activate(selected);
      return;
    }
    if (key === 'ArrowRight') {
      const row = rows.find(candidate => candidate.path === selected);
      if (row === undefined || row.kind !== 'dir') return;
      if (row.expanded) setSelected(moveSelection(rows, selected, 1));
      else setExpanded([...expanded, row.path]);
      return;
    }
    if (key === 'ArrowLeft' && selected !== null) {
      if (expanded.includes(selected)) setExpanded(expanded.filter(dir => dir !== selected));
      else if (selected.includes('/')) setSelected(selected.slice(0, selected.lastIndexOf('/')));
    }
  }

  return (
    <div data-pane="files-tree" tabIndex={0} onKeyDown={event => onKeyDown(event.key)}
      className="flex flex-col size-full bg-surface">
      <div className="flex-none p-1.5 border-b border-border">
        <div className="px-2 py-0.5 rounded-md bg-input-bg text-ui-12">
          <input type="text" value={filter} aria-label="Filter files" placeholder="Filter files…"
            onChange={event => setFilter(event.target.value)}
            onFocus={() => setFilterFocused(true)} onBlur={() => setFilterFocused(false)}
            className={filter === '' ? 'w-full text-ui-12 text-text-faint' : 'w-full text-ui-12 text-text'} />
        </div>
      </div>
      {rows.length > 0 ? null : (
        <div className="p-2.5 text-ui-12 text-text-faint">{filtering ? 'No matches' : 'Empty'}</div>
      )}
      <div role="tree" aria-label="Files" className="flex-1 min-h-0 overflow-y-scroll">
        {rows.map(row => renderTreeRow(row, selected === row.path, activate))}
      </div>
    </div>
  );
}
