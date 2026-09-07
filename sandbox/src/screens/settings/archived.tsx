// Rust: app/crates/ui/src/settings/archived.rs (ArchivedPage::render).
// Archived chats across devices, with Unarchive. The engine owns the mutation,
// so busy and error arrive as props; the hovered row is local view state, as it
// is in Rust (ArchivedPage::hovered drives the reveal).
import { useState } from 'react';
import { pageColumn, pageHeader, pageSubtitle, errorStrip } from './widgets';
import { renderIcon } from '../../icons';

export type ArchivedRow = {
  id: string;
  title: string;
  device: string | null;
  location: string | null;
  timeAgo: string;
};
export type ArchivedProps = {
  rows: readonly ArchivedRow[];
  busyId: string | null;
  error: string | null;
  onUnarchive: (chatId: string) => void;
  onDismissError: () => void;
};

function emptyState() {
  return (
    <div className="mt-24 flex flex-col items-center text-center text-text-muted/50">
      <span className="size-7 flex text-text-muted/14">{renderIcon('archive-minimalistic')}</span>
      <div className="mt-3 text-ui-14">Nothing archived</div>
      <div className="mt-1 text-ui-12 text-text-muted/50">Right-click a session in the sidebar to archive it.</div>
    </div>
  );
}

function metaLine(row: ArchivedRow) {
  return (
    <div className="mt-0.5 flex flex-row items-center gap-1.5 text-ui-11 text-text-muted/50">
      {row.device !== null && <span className="flex-none">{row.device}</span>}
      {row.device !== null && row.location !== null && <span className="flex-none">{'·'}</span>}
      {row.location !== null && <span className="min-w-0 truncate">{row.location}</span>}
    </div>
  );
}

export function ArchivedPage({ rows, busyId, error, onUnarchive, onDismissError }: ArchivedProps) {
  const [hovered, setHovered] = useState<string | null>(null);
  const count = rows.length;
  return (
    <div className="size-full overflow-y-scroll" data-page="archived">
      {pageColumn(
        <>
          {pageHeader('Archived sessions', count > 0 ? count : undefined)}
          {pageSubtitle('Hidden from the sidebar, never deleted. Unarchiving puts a session back on its device.')}
          {error !== null && errorStrip(error, onDismissError)}
          {count === 0 ? emptyState() : (
            <div className="mt-6 flex flex-col gap-0.5">
              {rows.map(row => {
                const busy = busyId === row.id;
                return (
                  <div key={row.id} data-row={row.id}
                    onMouseEnter={() => setHovered(row.id)} onMouseLeave={() => setHovered(null)}
                    className="flex flex-row items-center gap-3 rounded-lg px-3 py-2 hover:bg-wash/5">
                    <div className="flex-none size-8 rounded-md border border-border flex items-center justify-center">
                      <span className="size-4 flex text-text-muted/50">{renderIcon('archive-minimalistic')}</span>
                    </div>
                    <div className="flex-1 min-w-0 flex flex-col">
                      <div className="flex flex-row items-center gap-2">
                        <div className="min-w-0 truncate text-ui-13 font-medium text-text">{row.title}</div>
                        <div className="flex-none text-ui-11 text-text-muted/50">{row.timeAgo}</div>
                      </div>
                      {metaLine(row)}
                    </div>
                    <button type="button" data-action="unarchive" disabled={busy}
                      onClick={() => onUnarchive(row.id)}
                      className={busy
                        ? 'flex-none flex flex-row items-center gap-1.5 px-2.5 py-1 rounded-md border border-border text-ui-12 text-text-muted cursor-default opacity-50'
                        : hovered === row.id
                        ? 'flex-none flex flex-row items-center gap-1.5 px-2.5 py-1 rounded-md border border-border text-ui-12 text-text-muted cursor-pointer opacity-100 hover:bg-surface-raised hover:text-text active:bg-surface-raised-hover focus:bg-surface-raised focus:text-text'
                        : 'flex-none flex flex-row items-center gap-1.5 px-2.5 py-1 rounded-md border border-border text-ui-12 text-text-muted cursor-pointer opacity-0 hover:bg-surface-raised hover:text-text active:bg-surface-raised-hover focus:opacity-100 focus:bg-surface-raised focus:text-text'}>
                      <span className="flex-none size-3.5 flex">{renderIcon('archive-up-minimalistic')}</span>
                      {busy ? 'Unarchiving…' : 'Unarchive'}
                    </button>
                  </div>
                );
              })}
            </div>
          )}
        </>
      )}
    </div>
  );
}
