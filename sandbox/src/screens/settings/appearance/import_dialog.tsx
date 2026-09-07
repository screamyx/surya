// Rust: app/crates/ui/src/settings/appearance.rs, AppearancePage::render_import_dialog
// and the free function report_panel(). A 600px card with its own header, a
// scrolling body and a footer bar; the body is source, install mode, then the
// detected variants once a compile has run.
import type { ReactNode } from 'react';
import { compactAction, primaryButton } from './chips';
import { importScenePreview, palettePreview } from './previews';
import { renderIcon } from '../../../icons';
import { } from '../widgets';
import type { ImportReport, InstallMode, ThemeVariantOption } from '../appearance';

export type ImportDialogState = {
  source: string;
  mode: InstallMode;
  compiled?: readonly ThemeVariantOption[];
  reports?: Readonly<Record<string, ImportReport>>;
  failures?: readonly string[];
  selected: readonly string[];
  reviewVariant?: string;
  error?: string;
};

export type ImportDialogCallbacks = {
  onClose?: () => void;
  onBrowse?: () => void;
  onSetMode?: (mode: InstallMode) => void;
  onToggleVariant?: (variantId: string) => void;
  onToggleReview?: (variantId: string) => void;
  onSubmit?: () => void;
};

// Rust: report_panel(theme, report). A scrolling summary over its detail lines.
export function reportPanel(report: ImportReport) {
  return (
    <div className="mt-2 w-full max-h-40 overflow-y-scroll rounded-lg border border-border bg-surface-raised/50 p-2.5 text-ui-11 leading-normal text-text-muted">
      <div className="text-text">{report.summary}</div>
      {report.lines.map(line => <div key={line} className="mt-1">{line}</div>)}
    </div>
  );
}

// Rust: the `section_label` closure inside render_import_dialog.
function sectionLabel(label: string, flush = false) {
  return <div className={flush
    ? 'text-ui-11 font-semibold text-text-muted'
    : 'mb-2 text-ui-11 font-semibold text-text-muted'}>{label}</div>;
}

// Rust: the `mode_control` closure. A radio card per install mode.
function modeControl(
  label: string, description: string, value: InstallMode, active: boolean, onSelect?: () => void,
) {
  return (
    <button type="button" data-mode={value} aria-pressed={active} onClick={onSelect}
      className={active
        ? 'flex-1 min-w-0 p-2.5 rounded-lg border border-accent bg-accent-wash flex flex-col text-left cursor-pointer'
        : 'flex-1 min-w-0 p-2.5 rounded-lg border border-border bg-surface-raised/14 flex flex-col text-left cursor-pointer hover:bg-surface-raised-hover active:bg-surface-raised focus:bg-surface-raised-hover'}>
      <span className="flex flex-row items-center gap-2">
        <span className={active
          ? 'size-4 rounded-full border border-accent flex items-center justify-center'
          : 'size-4 rounded-full border border-border-strong flex items-center justify-center'}>
          {active && <span className="size-2 rounded-full bg-accent" />}
        </span>
        <span className={active ? 'text-ui-12 font-medium text-text' : 'text-ui-12 font-medium text-text-muted'}>{label}</span>
      </span>
      <span className="mt-1 ml-6 text-ui-10 text-text-faint">{description}</span>
    </button>
  );
}

// Rust: the per-variant row in the "Detected themes" list.
function variantRow(
  variant: ThemeVariantOption, selected: boolean, reviewOpen: boolean,
  report: ImportReport | undefined, callbacks: ImportDialogCallbacks,
) {
  return (
    <div key={variant.id} data-import-row={variant.id}
      className={selected
        ? 'mt-2 p-3 rounded-lg border border-accent/50 bg-accent-wash flex flex-col'
        : 'mt-2 p-3 rounded-lg border border-border bg-surface-raised/14 flex flex-col'}>
      <div className="flex flex-row items-center gap-2.25">
        <button type="button" role="checkbox" aria-checked={selected} data-select={variant.id}
          aria-label={`Import ${variant.name}`} onClick={() => callbacks.onToggleVariant?.(variant.id)}
          className={selected
            ? 'size-5 rounded-sm border border-accent bg-accent flex items-center justify-center cursor-pointer'
            : 'size-5 rounded-sm border border-border-strong bg-bg flex items-center justify-center cursor-pointer hover:border-accent active:bg-wash/10 focus:border-accent'}>
          {selected && <span className="size-3 flex text-on-accent">{renderIcon('check')}</span>}
        </button>
        {palettePreview()}
        <div className="flex-1 min-w-0">
          <div className="text-ui-12 font-medium text-text truncate">{variant.name}</div>
          <div className="text-ui-11 text-text-faint">{variant.appearance === 'dark' ? 'Dark' : 'Light'}</div>
        </div>
        {compactAction(`theme-import-review-${variant.id}`, reviewOpen ? 'Hide details' : 'Details',
          () => callbacks.onToggleReview?.(variant.id))}
      </div>
      {reviewOpen && (
        <div className="mt-2.5 pt-2.5 border-t border-border flex flex-col">
          {importScenePreview(variant)}
          {report !== undefined && reportPanel(report)}
        </div>
      )}
    </div>
  );
}

export function renderImportDialog(state: ImportDialogState, callbacks: ImportDialogCallbacks) {
  const compiled = state.compiled;
  const ready = compiled !== undefined && state.selected.length > 0;
  let body: ReactNode;
  if (compiled === undefined) {
    body = (
      <div className="mt-3.5 flex flex-row items-start gap-2 text-ui-11 leading-normal text-text-faint">
        <span className="size-3.5 flex-none mt-0.25 flex">{renderIcon('info-circle')}</span>
        <span>Surya finds light and dark variants automatically.</span>
      </div>
    );
  } else {
    body = (
      <>
        <div className="mt-4 pt-4 border-t border-border flex flex-row items-baseline justify-between">
          {sectionLabel('Detected themes', true)}
          <div className="text-ui-10 text-text-faint">
            {compiled.length} {compiled.length === 1 ? 'variant' : 'variants'}
          </div>
        </div>
        {compiled.map(variant => variantRow(
          variant, state.selected.includes(variant.id), state.reviewVariant === variant.id,
          state.reports?.[variant.id], callbacks,
        ))}
        {(state.failures ?? []).map(failure => (
          <div key={failure} className="mt-2 p-2.5 rounded-lg bg-warning/10 text-ui-11 text-warning">{failure}</div>
        ))}
      </>
    );
  }
  return (
    <div className="absolute top-0 left-0 size-full bg-bg/50 flex items-center justify-center">
      <div role="dialog" aria-label="Add a theme" data-dialog="theme-import"
        className="relative motion-dialog-in w-184 max-h-full rounded-xl bg-surface-dialog border border-border overflow-hidden flex flex-col text-text">
        <div className="px-5 pt-4 pb-4 flex flex-row items-start gap-4">
          <div className="flex-1 min-w-0">
            <div className="text-ui-16 font-semibold text-text">Add a theme</div>
            <p className="mt-1 text-ui-13 leading-normal text-text-muted">
              Import a local theme into your library or keep it linked to its source.
            </p>
          </div>
          <button type="button" data-action="theme-import-close" aria-label="Close" onClick={callbacks.onClose}
            className="size-7 rounded-md border border-border bg-surface-raised/14 flex items-center justify-center cursor-pointer hover:bg-surface-raised-hover active:bg-surface-raised focus:bg-surface-raised-hover">
            <span className="size-3 flex text-text-muted">{renderIcon('close')}</span>
          </button>
        </div>
        <div className="max-h-96 overflow-y-scroll px-5 pb-4 flex flex-col">
          {sectionLabel('Source')}
          <div className="flex flex-row items-center gap-2">
            <div className="flex-1 min-w-0 h-9 px-3 rounded-lg border border-border bg-wash/5 text-ui-14 flex items-center">
              <span className={state.source === '' ? 'truncate text-text-faint' : 'truncate text-text'}>
                {state.source === '' ? 'Path to a theme file' : state.source}
              </span>
            </div>
            {compactAction('theme-import-browse', 'Browse…', callbacks.onBrowse)}
          </div>
          <div className="mt-4 flex flex-col">
            {sectionLabel('Keep it up to date')}
            <div className="flex flex-row gap-2">
              {modeControl('Import a copy', 'Works independently from the original file.', 'snapshot',
                state.mode === 'snapshot', () => callbacks.onSetMode?.('snapshot'))}
              {modeControl('Link to source', 'Reload changes from the file on disk.', 'link',
                state.mode === 'link', () => callbacks.onSetMode?.('link'))}
            </div>
          </div>
          {body}
          {state.error !== undefined && (
            <div className="mt-3 p-2.5 rounded-lg bg-danger/10 flex flex-row items-start gap-2 text-ui-11 leading-normal text-danger">
              <span className="size-3.5 flex-none mt-0.25 flex">{renderIcon('danger-triangle')}</span>
              <span className="flex-1 min-w-0 truncate">{state.error}</span>
            </div>
          )}
        </div>
        <div className="border-t border-border bg-surface-raised/14 px-5 py-3 flex flex-row items-center justify-end gap-2">
          {compactAction('theme-import-cancel', 'Cancel', callbacks.onClose)}
          {primaryButton('theme-import-action', compiled === undefined ? 'Analyze theme' : 'Import selected',
            callbacks.onSubmit, compiled !== undefined && !ready)}
        </div>
      </div>
    </div>
  );
}
