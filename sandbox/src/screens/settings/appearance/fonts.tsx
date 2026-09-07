// Rust: app/crates/ui/src/settings/appearance.rs, the font_trigger / font_menu and
// size_trigger / size_menu blocks of AppearancePage::render. Keyboard stepping
// (on_font_key_down, on_size_key_down) stays in Rust; the sandbox keeps the
// open state and the pick.
import { renderIcon } from '../../../icons';
import { } from '../widgets';

export type FontChoice = { label: string; available: boolean };

function dropdownTrigger(id: string, width: 'wide' | 'narrow', label: string, open: boolean, onToggle: () => void) {
  return (
    <button type="button" data-action={id} aria-expanded={open} onClick={onToggle}
      className={open
        ? (width === 'wide'
          ? 'w-55 h-9 px-3 rounded-lg border border-border-strong bg-wash/5 flex flex-row items-center gap-2 cursor-pointer'
          : 'w-32 h-9 px-3 rounded-lg border border-border-strong bg-wash/5 flex flex-row items-center gap-2 cursor-pointer')
        : (width === 'wide'
          ? 'w-55 h-9 px-3 rounded-lg border border-border bg-wash/5 flex flex-row items-center gap-2 cursor-pointer hover:border-border-strong active:bg-wash/10 focus:border-border-strong'
          : 'w-32 h-9 px-3 rounded-lg border border-border bg-wash/5 flex flex-row items-center gap-2 cursor-pointer hover:border-border-strong active:bg-wash/10 focus:border-border-strong')}>
      <span className="flex-1 min-w-0 truncate text-ui-13 text-text text-left">{label}</span>
      <span className="size-3.5 flex-none flex text-text-muted">{renderIcon('alt-arrow-down')}</span>
    </button>
  );
}

export function renderFontPicker(
  choices: readonly FontChoice[], current: string, open: boolean,
  onToggle: () => void, onPick: (label: string) => void,
) {
  return (
    <div className="relative flex flex-col">
      {dropdownTrigger('interface-font-dropdown', 'wide', current, open, onToggle)}
      {open && (
        <div className="absolute top-10 left-0">
        <div className="relative motion-menu-in w-55 max-h-80 overflow-y-scroll p-1 rounded-lg border border-border bg-surface-overlay flex flex-col gap-0.5">
          {choices.map(choice => {
            const selected = choice.label === current;
            if (!choice.available) {
              return (
                <div key={choice.label} data-font={choice.label}
                  className="flex flex-row items-center gap-2.5 px-2 py-1.5 rounded-lg text-ui-13 text-text-muted opacity-50">
                  <span className="flex-1 min-w-0 truncate">{choice.label}</span>
                  <span className="w-5 flex-none" />
                </div>
              );
            }
            return (
              <button key={choice.label} type="button" data-font={choice.label} onClick={() => onPick(choice.label)}
                className={selected
                  ? 'flex flex-row items-center gap-2.5 px-2 py-1.5 rounded-lg text-ui-13 text-left cursor-pointer bg-element-active text-text'
                  : 'motion-hover-fade flex flex-row items-center gap-2.5 px-2 py-1.5 rounded-lg text-ui-13 text-left cursor-pointer text-text-muted hover:bg-element-hover hover:text-text active:bg-element-active focus:bg-element-hover'}>
                <span className="flex-1 min-w-0 truncate">{choice.label}</span>
                <span className="w-5 flex-none flex">
                  {selected && <span className="size-3.5 flex text-accent">{renderIcon('check')}</span>}
                </span>
              </button>
            );
          })}
        </div>
        </div>
      )}
    </div>
  );
}

export function renderSizePicker(
  sizes: readonly string[], current: string, open: boolean,
  onToggle: () => void, onPick: (size: string) => void,
) {
  return (
    <div className="relative flex flex-col">
      {dropdownTrigger('interface-font-size-dropdown', 'narrow', current, open, onToggle)}
      {open && (
        <div className="absolute top-10 right-0">
        <div className="relative motion-menu-in w-32 p-1 rounded-lg border border-border bg-surface-overlay flex flex-col gap-0.5">
          {sizes.map(size => {
            const selected = size === current;
            return (
              <button key={size} type="button" data-size={size} onClick={() => onPick(size)}
                className={selected
                  ? 'flex flex-row items-center gap-2.5 px-2 py-1.5 rounded-lg text-ui-13 text-left cursor-pointer bg-element-active text-text'
                  : 'motion-hover-fade flex flex-row items-center gap-2.5 px-2 py-1.5 rounded-lg text-ui-13 text-left cursor-pointer text-text-muted hover:bg-element-hover hover:text-text active:bg-element-active focus:bg-element-hover'}>
                <span className="flex-1">{size}</span>
                <span className="w-5 flex-none flex">
                  {selected && <span className="size-3.5 flex text-accent">{renderIcon('check')}</span>}
                </span>
              </button>
            );
          })}
        </div>
        </div>
      )}
    </div>
  );
}
