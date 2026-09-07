// Rust: app/crates/ui/src/settings/yolo_default.rs (section). A plain render
// function, mirroring the Rust file boundary. The setting itself stays in Rust
// (ui-settings.json, device-local); this emits onToggle.
// The card is spelled out rather than taken from widgets::sectionCard because
// the Rust section overrides its mt(24) with mt(16).
import { cardRow, metaLine, rowTile, rowTitle, toggleSwitch } from '../widgets';

export function yoloDefaultSection(on: boolean, onToggle: () => void) {
  return (
    <div className="mt-4 rounded-xl border border-border bg-surface-card overflow-hidden flex flex-col">
      {cardRow(true, <>
        {rowTile('danger-triangle')}
        <div className="flex-1 min-w-0 flex flex-col">
          {rowTitle('Yolo mode for new sessions')}
          {metaLine([
            <span key="blurb">
              New sessions run tools without asking. Sessions you already have
              keep their own setting; switch those in the composer.
            </span>,
          ])}
        </div>
        {toggleSwitch(on, 'Yolo mode for new sessions', onToggle)}
      </>)}
    </div>
  );
}
