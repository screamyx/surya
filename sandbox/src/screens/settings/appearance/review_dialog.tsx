// Rust: app/crates/ui/src/settings/appearance.rs, AppearancePage::render_review_dialog.
// A 660px scrolling card: every variant of one library entry with its scene
// preview and its import report.
import { primaryButton } from './chips';
import { importScenePreview } from './previews';
import { reportPanel } from './import_dialog';
import type { CustomThemeEntry } from '../appearance';

export function renderReviewDialog(entry: CustomThemeEntry, onClose?: () => void) {
  return (
    <div className="absolute top-0 left-0 size-full bg-bg/50 flex items-center justify-center">
      <div role="dialog" aria-label="Theme mapping" data-dialog="theme-review"
        className="relative motion-dialog-in w-184 max-h-full overflow-y-scroll p-5 rounded-xl bg-surface-dialog border border-border flex flex-col text-text">
        <div className="text-ui-16 font-semibold text-text">Theme mapping</div>
        <p className="mt-1.5 text-ui-13 leading-normal text-text-muted">
          {entry.name} · {entry.linked ? 'Linked source' : 'Imported copy'}
        </p>
        {entry.variants.map(variant => {
          const report = entry.reports?.[variant.id];
          return (
            <div key={variant.id} className="flex flex-col">
              <div className="mt-3.5 text-ui-12 font-medium text-text">{variant.name}</div>
              {importScenePreview(variant)}
              {report !== undefined && reportPanel(report)}
            </div>
          );
        })}
        <div className="mt-4 flex flex-row justify-end">
          {primaryButton('theme-review-close', 'Done', onClose)}
        </div>
      </div>
    </div>
  );
}
