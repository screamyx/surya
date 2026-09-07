// Rust: app/crates/ui/src/settings/appearance.rs free functions surface_label(),
// surface_helper(), surface_choice(), motion_choice(), motion_helper(),
// accent_helper(), accent_swatch() and compact_action().
import type { ReactNode } from 'react';
import type { AccentChoice, MotionMode, SurfacePreference } from '../appearance';

// Rust: popover::tracked_upper(label). gpui has no letter-spacing at the pinned
// revision, so a menu heading approximates 0.1em tracking with hair spaces; the
// sandbox whitelist has no tracking utility either, so it does the same.
export function trackedUpper(label: string) {
  return label.toUpperCase().split('').join('\u200A');
}

export const SURFACES: readonly SurfacePreference[] = ['theme-default', 'frosted', 'opaque'];
export const MOTION_MODES: readonly MotionMode[] = ['system', 'full', 'reduced'];
export const ACCENTS: readonly AccentChoice[] = [
  'theme-default', 'Surya', 'Orange', 'Amber', 'Green', 'Cyan', 'Blue', 'Pink',
];

// Rust: surface_label(surface).
export function surfaceLabel(surface: SurfacePreference) {
  if (surface === 'frosted') return 'Frosted';
  if (surface === 'opaque') return 'Opaque';
  return 'Theme default';
}

// Rust: surface_helper(surface, resolved).
export function surfaceHelper(surface: SurfacePreference, resolved: 'frosted' | 'opaque') {
  if (surface === 'frosted') return 'Theme-colored glass where supported.';
  if (surface === 'opaque') return 'Solid surfaces for every theme.';
  return `Uses this theme's ${resolved} default.`;
}

// Rust: motion_helper(mode). System says what the machine currently asks for.
export function motionHelper(mode: MotionMode, systemReduced: boolean) {
  if (mode === 'full') return 'Panels slide and fade at full travel.';
  if (mode === 'reduced') return 'Panels change state without travel. Feedback stays.';
  return systemReduced
    ? 'Following this machine, which asks for reduced motion.'
    : 'Following this machine, which asks for full motion.';
}

// Rust: motion_choice's MotionMode::label().
export function motionLabel(mode: MotionMode) {
  if (mode === 'full') return 'Full';
  if (mode === 'reduced') return 'Reduced';
  return 'System';
}

// Rust: accent_helper(accent).
export function accentHelper(accent: AccentChoice) {
  return accent === 'theme-default'
    ? "Theme default · Uses the palette's intended color."
    : `${accent} · Controls, glyphs, selections, code, and activity.`;
}

// Rust: surface_choice(theme, surface, selected) and motion_choice(theme, mode,
// selected). They are the same 30px chip: the Motion row sits under Glass and a
// second chip shape there would read as two controls doing the same job.
export function choiceChip(id: string, label: string, selected: boolean, onSelect?: () => void) {
  return (
    <button key={id} type="button" data-choice={id} aria-pressed={selected} onClick={onSelect}
      className={selected
        ? 'h-7.5 px-2.5 rounded-md border border-accent bg-accent-wash text-ui-11 font-medium text-accent flex items-center cursor-pointer'
        : 'h-7.5 px-2.5 rounded-md border border-border bg-surface-raised/14 text-ui-11 text-text-muted flex items-center cursor-pointer hover:bg-surface-raised-hover hover:text-text active:bg-surface-raised focus:bg-surface-raised-hover'}>
      {label}
    </button>
  );
}

// Rust: accent_swatch(page_theme, selection, selected). The Theme default swatch
// shows three glyph bars over the accent wash; a preset shows its flat accent.
// The Rust builds each swatch from that selection's resolved theme, so every
// swatch paints its own hue. The sandbox has one accent token, so the seven
// presets all paint the live accent and only selection differs.
export function accentSwatch(accent: AccentChoice, selected: boolean, onSelect?: () => void) {
  const sample = accent === 'theme-default' ? (
    <span className="size-full rounded-md bg-accent-wash flex items-center justify-center gap-0.5">
      <span className="w-1 h-3 rounded-sm bg-accent/50" />
      <span className="w-1 h-4 rounded-sm bg-accent" />
      <span className="w-1 h-3 rounded-sm bg-accent/50" />
    </span>
  ) : <span className="size-full rounded-md bg-accent flex" />;
  return (
    <button key={accent} type="button" data-accent={accent} aria-pressed={selected} onClick={onSelect}
      aria-label={accent === 'theme-default' ? 'Theme default accent' : `${accent} accent`}
      className={selected
        ? 'flex-none w-7.5 h-8 pb-1 border-b border-accent cursor-pointer flex'
        : 'flex-none w-7.5 h-8 pb-1 border-b border-bg cursor-pointer flex hover:border-border'}>
      <span className={selected
        ? 'size-7.5 p-0.5 rounded-lg border border-border-strong bg-surface-raised/50 flex'
        : 'size-7.5 p-0.5 rounded-lg border border-border bg-surface-raised/50 flex'}>{sample}</span>
    </button>
  );
}

// Rust: compact_action(theme, label, id). popover::btn_ghost in a bordered
// 28px pill, used by the library rows and the import dialog.
export function compactAction(id: string, label: string, onClick?: () => void, danger = false) {
  return (
    <button key={id} type="button" data-action={id} onClick={onClick}
      className={danger
        ? 'motion-hover-fade h-7 px-2.25 rounded-md border border-border bg-surface-raised/50 text-ui-11 text-danger flex items-center cursor-pointer hover:bg-danger/10 active:bg-danger/14 focus:bg-danger/10'
        : 'motion-hover-fade h-7 px-2.25 rounded-md border border-border bg-surface-raised/50 text-ui-11 text-text-muted flex items-center cursor-pointer hover:bg-wash/5 hover:text-text active:bg-wash/10 focus:bg-wash/5 focus:text-text'}>
      {label}
    </button>
  );
}

// Rust: popover::btn_primary(theme, label). White fill, near-black text.
export function primaryButton(id: string, label: ReactNode, onClick?: () => void, dim = false) {
  return (
    <button type="button" data-action={id} onClick={onClick}
      className={dim
        ? 'px-3 py-1.5 rounded-lg bg-text text-ui-13 font-medium text-on-solid cursor-pointer opacity-50'
        : 'px-3 py-1.5 rounded-lg bg-text text-ui-13 font-medium text-on-solid cursor-pointer hover:bg-text/88 active:bg-text/50 focus:bg-text/88'}>
      {label}
    </button>
  );
}
