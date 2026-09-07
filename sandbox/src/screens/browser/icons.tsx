// Rust: app/crates/ui/assets/icons/{refresh,magnifer,arrow-up,arrow-down}.svg,
// named from app/crates/ui/src/icons.rs REFRESH, MAGNIFER, ARROW_UP, ARROW_DOWN.
// The generated src/shell/icons.tsx carries only the sixteen names the shell
// uses, and this pane needs four more. Same geometry, copied from the assets.
const icons = {
  refresh: (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><g fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"><path d="M19.73 11a8 8 0 1 1-2.07-4.66"/><path strokeLinejoin="round" d="M17.16 3.34l.63 2.86l-2.86.63"/></g></svg>),
  magnifer: (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><g fill="none" stroke="currentColor" strokeWidth="1.5"><circle cx="11.5" cy="11.5" r="9.5"/><path strokeLinecap="round" d="M18.5 18.5L22 22"/></g></svg>),
  'arrow-up': (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><path fill="none" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.5" d="M12 20V4m0 0l6 6m-6-6l-6 6"/></svg>),
  'arrow-down': (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><path fill="none" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.5" d="M12 4v16m0 0l6-6m-6 6l-6-6"/></svg>),
};
export function renderBrowserIcon(name: keyof typeof icons) { return icons[name]; }
