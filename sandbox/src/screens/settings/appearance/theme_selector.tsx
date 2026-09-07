// Rust: app/crates/ui/src/settings/appearance.rs, AppearancePage::render_theme_selector.
// The 218px trigger and the dropdown of every registered variant for one
// appearance. The Rust owns the open state on the page entity, so the page here
// passes `open` in and the toggle out.
import { palettePreview } from './previews';
import { trackedUpper } from './chips';
import { pageIcon } from './icons';
import { settingsIcon } from '../widgets';
import type { ThemeVariantOption } from '../appearance';

export function renderThemeSelector(
  kind: 'light' | 'dark',
  variants: readonly ThemeVariantOption[],
  selectedId: string,
  open: boolean,
  onToggle: () => void,
  onPick: (variantId: string) => void,
) {
  const selected = variants.find(variant => variant.id === selectedId) ?? variants[0];
  return (
    <div className="relative flex-none flex flex-col">
      <button type="button" data-action={`${kind}-theme-selector`} aria-expanded={open} onClick={onToggle}
        className={open
          ? 'flex-none w-55 h-8 px-2.5 rounded-lg border border-border-strong bg-surface-raised/88 flex flex-row items-center gap-2 cursor-pointer'
          : 'flex-none w-55 h-8 px-2.5 rounded-lg border border-border bg-surface-raised/50 flex flex-row items-center gap-2 cursor-pointer hover:bg-surface-raised-hover active:bg-surface-raised focus:bg-surface-raised-hover'}>
        {palettePreview()}
        <span className="flex-1 min-w-0 truncate text-ui-12 font-medium text-text text-left">{selected?.name ?? 'Theme'}</span>
        <span className={open ? 'size-3.5 flex-none flex text-text-muted' : 'size-3.5 flex-none flex text-text-faint/50'}>
          {pageIcon('sort-vertical')}
        </span>
      </button>
      {open && (
        <div className="absolute top-9 right-0 w-64 p-1 rounded-lg border border-border bg-surface-overlay flex flex-col gap-0.5 text-ui-13 text-text">
          <div className="px-2 pb-1 pt-1.5 text-ui-10 font-medium text-text-faint">
            {kind === 'light' ? trackedUpper('Light themes') : trackedUpper('Dark themes')}
          </div>
          {variants.map(variant => {
            const active = variant.id === selectedId;
            return (
              <button key={variant.id} type="button" data-variant={variant.id} onClick={() => onPick(variant.id)}
                className={active
                  ? 'flex flex-row items-center gap-2.5 px-2 py-1.5 rounded-lg text-ui-13 text-left cursor-pointer bg-element-active text-text'
                  : 'flex flex-row items-center gap-2.5 px-2 py-1.5 rounded-lg text-ui-13 text-left cursor-pointer text-text-muted hover:bg-element-hover hover:text-text active:bg-element-active focus:bg-element-hover'}>
                {palettePreview()}
                <span className="flex-1 min-w-0 truncate">{variant.name}</span>
                {active && <span className="size-3.5 flex-none flex text-accent">{settingsIcon('check')}</span>}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
