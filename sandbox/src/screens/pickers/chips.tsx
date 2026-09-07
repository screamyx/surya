// Rust: app/crates/ui/src/pickers.rs - trigger_chip (2174), footer_chip (2286),
// footer_label (2342); and app/crates/ui/src/yolo.rs - chip (132).
import type { ReactNode } from 'react';
import { icon } from './popover';
import type { IconName } from './icons';
import type { YoloState } from '../../fixtures/pickers';

/// trigger_chip: the ghost pill in the composer's actions row. Brand mark, the
/// model name, then the muted traits summary as the chip's second tone.
export function triggerChip(
  id: string, label: string, set: boolean, chipIcon: IconName | null, brand: boolean,
  iconLoading: boolean, labelLoading: boolean, suffix: string | null, suffixActive: boolean,
  open: boolean, onClick: () => void,
) {
  return (
    <button type="button" data-chip={id} onClick={onClick} aria-expanded={open}
      className={open
        ? 'h-8 max-w-64 min-w-0 flex flex-row items-center gap-1.5 px-2.5 rounded-lg text-ui-12 font-medium cursor-pointer motion-hover-fade bg-element-hover text-text'
        : set
          ? 'h-8 max-w-64 min-w-0 flex flex-row items-center gap-1.5 px-2.5 rounded-lg text-ui-12 font-medium cursor-pointer motion-hover-fade text-text/88 hover:bg-element-hover hover:text-text active:bg-element-active focus:bg-element-hover'
          : 'h-8 max-w-64 min-w-0 flex flex-row items-center gap-1.5 px-2.5 rounded-lg text-ui-12 font-medium cursor-pointer motion-hover-fade text-text-muted hover:bg-element-hover hover:text-text active:bg-element-active focus:bg-element-hover'}>
      {iconLoading && <span className="flex-none w-4 h-2.5 rounded-md bg-wash/10 opacity-55" />}
      {!iconLoading && chipIcon && (
        <span className={brand ? 'flex-none text-claude-brand' : 'flex-none text-text-muted'}>{icon(chipIcon, 16)}</span>
      )}
      {labelLoading
        ? <span className="flex-none w-14 h-2.5 rounded-md bg-wash/10 opacity-55" />
        : <span className="min-w-0 truncate">{label}</span>}
      {suffix && (
        <span className={suffixActive ? 'min-w-0 truncate text-text/88' : 'min-w-0 truncate text-text-faint'}>{suffix}</span>
      )}
    </button>
  );
}

/// yolo::chip: how the run behaves, left of the run identity. On reads in the
/// warning family; Always is not a toggle and never brightens.
export function yoloChip(state: YoloState, onToggle: () => void) {
  const suffix = state === 'on' ? 'on' : state === 'always' ? 'always' : null;
  return (
    <button type="button" data-chip="picker-yolo" onClick={onToggle} disabled={state === 'always'}
      className={state === 'on'
        ? 'h-8 flex-none flex flex-row items-center gap-1.5 px-2.5 rounded-lg text-ui-12 font-medium cursor-pointer bg-warning-wash text-warning hover:text-text'
        : state === 'always'
          ? 'h-8 flex-none flex flex-row items-center gap-1.5 px-2.5 rounded-lg text-ui-12 font-medium cursor-default text-text-faint'
          : 'h-8 flex-none flex flex-row items-center gap-1.5 px-2.5 rounded-lg text-ui-12 font-medium cursor-pointer motion-hover-fade text-text-muted hover:bg-element-hover hover:text-text active:bg-element-active focus:bg-element-hover'}>
      {icon('danger-triangle', 16)}
      <span>Yolo</span>
      {suffix && <span className="text-text-faint">{suffix}</span>}
    </button>
  );
}

/// footer_chip: a footer-row trigger. Leading icon, truncating label, trailing
/// chevron - smaller and quieter than the in-pill chips.
export function footerChip(
  id: string, iconName: IconName, label: string, open: boolean, offline: boolean, onClick: () => void,
) {
  return (
    <button type="button" data-chip={id} onClick={onClick} aria-expanded={open}
      className={open
        ? 'h-5 max-w-64 min-w-0 flex flex-row items-center gap-1.5 px-2 rounded-md text-ui-12 font-medium cursor-pointer motion-hover-fade bg-element-hover text-text/88'
        : offline
          ? 'h-5 max-w-64 min-w-0 flex flex-row items-center gap-1.5 px-2 rounded-md text-ui-12 font-medium cursor-pointer motion-hover-fade text-warning hover:bg-element-hover active:bg-element-active focus:bg-element-hover'
          : 'h-5 max-w-64 min-w-0 flex flex-row items-center gap-1.5 px-2 rounded-md text-ui-12 font-medium cursor-pointer motion-hover-fade text-text-faint hover:bg-element-hover hover:text-text/88 active:bg-element-active focus:bg-element-hover'}>
      {icon(iconName, 12)}
      <span className="min-w-0 truncate">{label}</span>
      {icon('alt-arrow-down', 12)}
    </button>
  );
}

/// footer_label: the read-only footer label a locked session shows instead of a
/// chip. Four share one row, so each caps early and is allowed to shrink.
export function footerLabel(iconName: IconName, label: string) {
  return (
    <div className="h-5 max-w-40 min-w-0 flex flex-row items-center gap-1.5 px-2 text-ui-12 font-medium text-text-faint">
      {icon(iconName, 12)}
      <span className="min-w-0 truncate">{label}</span>
    </div>
  );
}

/// The floating layer a trigger carries while its popover is open. Every picker
/// mounts with anchored_menu_above (the composer sits at the bottom of the
/// canvas), pt-1.5 of air between the card and the trigger.
///
/// The outer div is the pinned+anchored layer; the inner one is the div
/// anchored_menu_above animates, `div().occlude().pb(px(6.0))`. They have to
/// stay apart: menu-in drives `top`, and an element already placed by `bottom`
/// would stretch between the two edges instead of sliding.
export function overlayAbove(align: 'start' | 'end', children: ReactNode) {
  return (
    <div className={align === 'start' ? 'absolute bottom-8 left-0' : 'absolute bottom-8 right-0'}>
      <div className="relative pb-1.5 motion-menu-in">{children}</div>
    </div>
  );
}

/// overlayAbove over the shorter h-5 footer chips.
export function overlayAboveFooter(align: 'start' | 'end', children: ReactNode) {
  return (
    <div className={align === 'start' ? 'absolute bottom-5 left-0' : 'absolute bottom-5 right-0'}>
      <div className="relative pb-1.5 motion-menu-in">{children}</div>
    </div>
  );
}
