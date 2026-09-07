// Rust: app/crates/ui/src/icons.rs, assets app/crates/ui/assets/icons/{cloud,tag,refresh}.svg.
// scripts/gen-icons.mjs emits only the shell's icon set, and src/shell/icons.tsx is
// generated output this seat may not extend. The three glyphs this screen needs are
// copied here verbatim from the same native assets, paint left as currentColor.
export const cloudIcon = (
  <svg viewBox="0 0 24 24" fill="none" className="size-full" aria-hidden="true">
    <path d="M7.25 18.5h10.1a4.15 4.15 0 0 0 .45-8.27A6.25 6.25 0 0 0 5.9 8.85a4.85 4.85 0 0 0 1.35 9.65Z" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
  </svg>
);
export const tagIcon = (
  <svg viewBox="0 0 24 24" fill="none" className="size-full" aria-hidden="true">
    <path d="M3.5 11.25V6.5a3 3 0 0 1 3-3h4.75a3 3 0 0 1 2.12.88l6.25 6.25a2 2 0 0 1 0 2.83l-6.16 6.16a2 2 0 0 1-2.83 0l-6.25-6.25a3 3 0 0 1-.88-2.12Z" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
    <circle cx="8.25" cy="8.25" r="1.25" stroke="currentColor" strokeWidth="1.5" />
  </svg>
);
export const refreshIcon = (
  <svg viewBox="0 0 24 24" className="size-full" aria-hidden="true">
    <g fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
      <path d="M19.73 11a8 8 0 1 1-2.07-4.66" />
      <path strokeLinejoin="round" d="M17.16 3.34l.63 2.86l-2.86.63" />
    </g>
  </svg>
);
