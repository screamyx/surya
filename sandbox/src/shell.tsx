// Rust: app/crates/ui/src/shell.rs (Shell); only the Needs you frame is ported.
// Rail and titlebar render functions are split for the 500-line cap.
import type { NeedsYouProps } from './screens/NeedsYou';
import type { Fixture } from './fixtures';
import { NeedsYouPane } from './screens/NeedsYou';
import { renderSidebar } from './shell/rail';
import { renderTitlebar } from './shell/titlebar';
export type ShellProps = NeedsYouProps & {
  fixture: Fixture; sidebarOpen: boolean; onEvent: (action: string) => void;
};
export function Shell({ rows, onOpenChat, fixture, sidebarOpen, onEvent }: ShellProps) {
  return (
    <div className="size-full relative flex flex-row bg-surface text-text font-sans leading-gpui">
      {sidebarOpen && renderSidebar(rows.length, fixture, onEvent)}
      <main className="flex-1 min-w-0 h-full pt-9.5">
        <NeedsYouPane rows={rows} onOpenChat={onOpenChat} />
      </main>
      {renderTitlebar(fixture, sidebarOpen, onEvent)}
    </div>
  );
}
