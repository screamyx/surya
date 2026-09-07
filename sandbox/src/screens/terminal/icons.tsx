// Rust: app/crates/ui/assets/icons/terminal.svg, used by
// app/crates/ui/src/terminal/panel.rs::render_tab_bar as icons::TERMINAL.
// Geometry copied from the native asset and put through the same two rewrites
// scripts/gen-icons.mjs applies (drop xmlns/width/height, camel-case the
// hyphenated paint attributes). It lives here because that generator's `names`
// list does not carry 'terminal' yet and scripts/ is not this seat's to edit;
// add the name there and this file goes away.
export function renderTerminalIcon() {
  return (
    <svg viewBox="0 0 16 16" fill="none" className="size-full" aria-hidden="true">
      <rect x="1.75" y="2.25" width="12.5" height="11.5" rx="3.25" stroke="currentColor" strokeWidth="1.25" />
      <path d="M4.75 6.1 6.65 8l-1.9 1.9" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round" />
      <path d="M8.4 10.4h2.85" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" />
    </svg>
  );
}
