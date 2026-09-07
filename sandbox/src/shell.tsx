// Rust: app/crates/ui/src/shell.rs (Shell).
// Rail and titlebar render functions are split for the 500-line cap.
// The rail routes, as shell.rs does: the frame stays, the outlet changes.
import type { ReactNode } from 'react';
import type { NeedsYouProps } from './screens/NeedsYou';
import type { Fixture } from './fixtures';
import { renderSidebar } from './shell/rail';
import { renderTitlebar } from './shell/titlebar';
export type Destination = 'chat' | 'needs-you' | 'inbox' | 'tasks' | 'files' | 'browser'
  | 'changes' | 'settings';
export type ShellProps = NeedsYouProps & {
  fixture: Fixture; sidebarOpen: boolean; onEvent: (action: string) => void;
  active: Destination; onNavigate: (to: Destination) => void; children: ReactNode;
};
export function Shell({ rows, fixture, sidebarOpen, onEvent, active, onNavigate, children }: ShellProps) {
  return (
    <div className="size-full relative flex flex-row bg-surface text-text font-sans leading-gpui">
      {sidebarOpen && renderSidebar(rows.length, fixture, onEvent, active, onNavigate)}
      <main className="flex-1 min-w-0 h-full pt-9.5">{children}</main>
      {renderTitlebar(fixture, sidebarOpen, onEvent, onNavigate)}
    </div>
  );
}
