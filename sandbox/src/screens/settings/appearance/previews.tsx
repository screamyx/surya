// Rust: app/crates/ui/src/settings/appearance.rs free functions bar(), miniature(),
// miniature_split(), preview(), palette_preview() and import_scene_preview().
// These are pure render helpers on the page, not components.
import type { ThemeVariantOption } from '../appearance';

// The Rust bar takes a relative width. The whitelist has no fractional widths,
// so the four rail bars and four page bars use the nearest fixed steps.
function railBars() {
  return (
    <>
      <div className="w-5 h-1 rounded-sm bg-text/50" />
      <div className="w-7 h-1 rounded-sm bg-text/14" />
      <div className="w-6 h-1 rounded-sm bg-text/14" />
      <div className="w-7 h-1 rounded-sm bg-text/14" />
    </>
  );
}

function pageBars() {
  return (
    <>
      <div className="w-20 h-1 rounded-sm bg-text/50" />
      <div className="w-28 h-1 rounded-sm bg-text/14" />
      <div className="w-24 h-1 rounded-sm bg-text/14" />
      <div className="w-16 h-1 rounded-sm bg-text/14" />
    </>
  );
}

export type Corners = 'all' | 'left' | 'right';

// Rust: miniature(theme, corners). A rail of four bars beside a bordered page
// of four bars. The Rust paints this with the *previewed* theme's palette; the
// sandbox has only the live theme's tokens, so both halves paint the same.
export function miniature(corners: Corners) {
  return (
    <div className={corners === 'all'
      ? 'size-full flex flex-row bg-surface rounded-md'
      : (corners === 'left' ? 'size-full flex flex-row bg-surface' : 'size-full flex flex-row bg-surface')}>
      <div className="w-11 h-full flex-none overflow-hidden flex flex-col gap-2 px-2 pt-3.5">
        {railBars()}
      </div>
      <div className="flex-1 min-w-0 my-2 mr-2 rounded-md border border-border bg-bg overflow-hidden flex flex-col gap-2 p-2.5">
        {pageBars()}
      </div>
    </div>
  );
}

// Rust: miniature_split(themes, accent, surface). The System card: light on the
// left, dark on the right. Both halves paint from the live theme here.
export function miniatureSplit() {
  return (
    <div className="size-full flex flex-row">
      <div className="flex-1 min-w-0 h-full overflow-hidden flex">{miniature('left')}</div>
      <div className="flex-1 min-w-0 h-full overflow-hidden flex">{miniature('right')}</div>
    </div>
  );
}

// Rust: preview(mode, themes, accent, surface).
export function preview(mode: 'system' | 'light' | 'dark') {
  return mode === 'system' ? miniatureSplit() : miniature('all');
}

// Rust: palette_preview(theme). Three stripes: surface, bg, accent. The stripes
// are the live theme's, not the previewed variant's - see the report's gaps.
export function palettePreview() {
  return (
    <div className="flex-none w-7.5 h-5 rounded-sm overflow-hidden border border-border flex flex-row">
      <div className="flex-1 h-full bg-surface" />
      <div className="flex-1 h-full bg-bg" />
      <div className="flex-1 h-full bg-accent" />
    </div>
  );
}

// Rust: import_scene_preview(variant). A miniature, a code sample over an ANSI
// strip, and a diff column. The Rust paints it from the compiled variant; the
// sandbox paints the live theme's syntax-adjacent tokens.
export function importScenePreview(variant: ThemeVariantOption) {
  return (
    <div data-scene={variant.id} className="w-full h-20 flex flex-row gap-2">
      <div className="w-40 h-full overflow-hidden rounded-lg border border-border flex">{miniature('all')}</div>
      <div className="flex-1 min-w-0 h-full rounded-lg border border-border bg-bg p-2.25 flex flex-col gap-1.5">
        <div className="text-ui-10 text-accent">
          fn <span className="text-code-text">preview</span><span className="text-text-muted">() {'{'}</span>
        </div>
        <div className="text-ui-10 text-success">{'  "Theme mapping"'}</div>
        {/* Rust: theme.terminal.ansi.iter().take(8). The sandbox has no terminal
            palette, so this is the eight most distinct admitted tokens in the
            same black/red/green/yellow/blue/magenta/white/grey order. */}
        <div className="mt-auto h-3 flex flex-row rounded-sm overflow-hidden">
          <div className="flex-1 h-full bg-bg" /><div className="flex-1 h-full bg-danger" />
          <div className="flex-1 h-full bg-success" /><div className="flex-1 h-full bg-warning" />
          <div className="flex-1 h-full bg-accent" /><div className="flex-1 h-full bg-claude-brand" />
          <div className="flex-1 h-full bg-text" /><div className="flex-1 h-full bg-surface-raised" />
        </div>
      </div>
      <div className="w-20 h-full rounded-lg border border-border bg-surface p-2 flex flex-col gap-1.5">
        <div className="h-3 rounded-sm bg-diff-add/50" />
        <div className="h-3 rounded-sm bg-diff-del/50" />
        <div className="h-3 rounded-sm bg-accent-wash" />
      </div>
    </div>
  );
}
