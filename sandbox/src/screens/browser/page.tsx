// Rust: app/crates/ui/src/browser_pane/mod.rs (off_pane, note_colors) and the
// page area of BrowserPane::render.
// Render helpers on the same pane, not second stateful components.

// The words the crate publishes when there is no page
// (app/crates/browser/src/off.rs OffNote).
export type OffNote = { label: string; line: string; hint: string | null };

// The note itself, on the shell's own surface. note_colors() deliberately
// keeps this off the white page plane and off text_faint: every line of it has
// to clear WCAG AA, because it is the only thing telling a person why the
// browser is empty and how to get it back. The hint takes the same tone as the
// sentence and is separated by the gap alone.
export function offPane(note: OffNote) {
  return (
    <div className="size-full flex flex-col gap-1.5 p-3.5 text-ui-12 bg-surface">
      <div className="text-text">{note.label}</div>
      <div className="text-text-muted">{note.line}</div>
      {note.hint !== null && <div className="text-text-muted">{note.hint}</div>}
    </div>
  );
}

// Where the offscreen Chromium composites into the gpui window. There is no
// page here: the sandbox bans iframes and a CEF view has no admitted pair, so
// the area is painted as a plain surface naming what it stands in for.
export function pagePlaceholder(address: string) {
  return (
    <div className="size-full flex flex-col items-center justify-center gap-1.5 bg-surface-raised">
      <span className="text-ui-13 text-text">Page area</span>
      {address !== '' && <span className="text-ui-12 text-text-muted">{address}</span>}
      <span className="text-ui-11 text-text-muted">The native pane composites Chromium here.</span>
    </div>
  );
}

// The load's progress as a hairline under the bar, gone at 1. GPUI sets the
// fill with relative(), a percentage the whitelist has no step for, so the
// fill takes the nearest admitted fixed width instead.
export function hairline(loading: boolean, progress: number) {
  if (!loading) return <div className="flex-none h-0.5 w-full" />;
  return (
    <div className="flex-none h-0.5 w-full">
      <div className={progress >= 0.9 ? 'h-full w-184 bg-text-muted'
        : progress >= 0.7 ? 'h-full w-96 bg-text-muted'
        : progress >= 0.5 ? 'h-full w-80 bg-text-muted'
        : progress >= 0.3 ? 'h-full w-64 bg-text-muted'
        : progress >= 0.15 ? 'h-full w-48 bg-text-muted'
        : 'h-full w-32 bg-text-muted'} />
    </div>
  );
}
