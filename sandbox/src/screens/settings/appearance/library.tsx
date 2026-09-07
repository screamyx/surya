// Rust: app/crates/ui/src/settings/appearance.rs, AppearancePage::render_library_entry
// and render_theme_library_rows. The library rows extend the same section card as
// the theme, accent, glass and motion rows above them.
import type { ReactNode } from 'react';
import { compactAction, primaryButton } from './chips';
import { pageIcon } from './icons';
import { cardRow, metaLine, rowTitle, settingsIcon } from '../widgets';
import type { CustomThemeEntry } from '../appearance';

export type LibraryCallbacks = {
  onAddTheme?: () => void;
  onReloadTheme?: (id: string) => void;
  onRevealTheme?: (id: string) => void;
  onReviewTheme?: (id: string) => void;
  onDuplicateTheme?: (id: string) => void;
  onUnlinkTheme?: (id: string) => void;
  onRemoveTheme?: (id: string) => void;
};

// Rust: render_library_entry(entry, theme, cx). Reload and Unlink appear on
// linked entries only; Remove is the one red action.
function renderLibraryEntry(entry: CustomThemeEntry, callbacks: LibraryCallbacks) {
  return cardRow(false, (
    <>
      <div className="flex-none size-9 rounded-lg border border-border bg-wash/5 flex items-center justify-center">
        <span className="size-4 flex text-text-muted">
          {entry.linked ? settingsIcon('global') : pageIcon('document')}
        </span>
      </div>
      <div className="flex-1 min-w-0">
        {rowTitle(entry.name)}
        <div className={entry.status === 'warning'
          ? 'truncate text-ui-11 text-warning'
          : 'truncate text-ui-11 text-text-muted'}>{entry.statusLine}</div>
      </div>
      <div className="flex-none flex items-center gap-0.5">
        {entry.linked && compactAction(`theme-reload-${entry.id}`, 'Reload', () => callbacks.onReloadTheme?.(entry.id))}
        {compactAction(`theme-reveal-${entry.id}`, 'Reveal', () => callbacks.onRevealTheme?.(entry.id))}
        {compactAction(`theme-review-${entry.id}`, 'Review', () => callbacks.onReviewTheme?.(entry.id))}
        {compactAction(`theme-duplicate-${entry.id}`, 'Duplicate as editable', () => callbacks.onDuplicateTheme?.(entry.id))}
        {entry.linked && compactAction(`theme-unlink-${entry.id}`, 'Unlink', () => callbacks.onUnlinkTheme?.(entry.id))}
        {compactAction(`theme-remove-${entry.id}`, 'Remove', () => callbacks.onRemoveTheme?.(entry.id), true)}
      </div>
    </>
  ));
}

// Rust: render_theme_library_rows(theme, cx). The "Theme library" row, then an
// IMPORTED group and a LINKED group, each behind its own hairline caption.
export function renderThemeLibraryRows(
  library: readonly CustomThemeEntry[], callbacks: LibraryCallbacks,
): ReactNode[] {
  const imported = library.filter(entry => !entry.linked);
  const linked = library.filter(entry => entry.linked);
  const rows: ReactNode[] = [
    <div key="theme-library">{cardRow(false, (
      <>
        <div className="flex-none size-9 rounded-lg border border-border bg-wash/5 flex items-center justify-center">
          <span className="size-4 flex text-text-muted">{pageIcon('folder-with-files')}</span>
        </div>
        <div className="flex-1 min-w-0">
          {rowTitle('Theme library')}
          {metaLine(['Import or link custom themes.'])}
        </div>
        {primaryButton('theme-library-add', 'Add theme', callbacks.onAddTheme)}
      </>
    ))}</div>,
  ];
  if (imported.length > 0) {
    rows.push(
      <div key="imported-caption"
        className="px-4 pt-3 pb-1 border-t border-border text-ui-10 font-semibold text-text-faint">IMPORTED</div>,
    );
    imported.forEach(entry => rows.push(<div key={entry.id}>{renderLibraryEntry(entry, callbacks)}</div>));
  }
  if (linked.length > 0) {
    rows.push(
      <div key="linked-caption"
        className="px-4 pt-3 pb-1 border-t border-border text-ui-10 font-semibold text-text-faint">LINKED</div>,
    );
    linked.forEach(entry => rows.push(<div key={entry.id}>{renderLibraryEntry(entry, callbacks)}</div>));
  }
  return rows;
}
