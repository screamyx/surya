// Rust: app/crates/ui/src/inbox/needs_you.rs (NeedsYouPane::render).
import { renderRow } from '../../../src/screens/needs_you/rows';
export type InboxRow = { id: string; chatId: string; prompt: string };
export type NeedsYouProps = { rows: readonly InboxRow[]; onOpenChat: (chatId: string) => void };
export function NeedsYouPane({ rows, onOpenChat }: NeedsYouProps) {
  return (
    <section aria-labelledby="needs-you-heading" className="size-full overflow-y-scroll flex flex-col items-center px-12 pb-8">
      <div className="w-full max-w-184 min-w-0 flex flex-col">
        <div className="flex flex-row items-baseline gap-2 pt-6 pb-3">
          <h1 id="needs-you-heading" className="text-ui-20 font-semibold text-text">Needs you</h1>
          {rows.length > 0 && <span className="text-ui-13 text-text-faint">{rows.length} waiting</span>}
        </div>
        {rows.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-10 text-ui-13 text-text-faint">Nothing needs you.</div>
        ) : rows.map((row, index) => renderRow(row, index === 0, onOpenChat))}
      </div>
    </section>
  );
}
