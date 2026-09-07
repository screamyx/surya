// Rust: app/crates/ui/src/settings/widgets.rs (the shared settings widget kit).
// Plain render helpers, in the same order and under the same names as the Rust
// module. Each returns a node; where the Rust returns a Div the caller fills,
// the React helper takes children. The icon map is the React stand-in for
// crate::icons::icon(path); it is keyed by the Rust asset name.
import type { ReactNode } from 'react';
const icons = {
  'monitor': (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><g fill="none" stroke="currentColor" strokeWidth="1.5"><path d="M2 10c0-3.771 0-5.657 1.172-6.828S6.229 2 10 2h4c3.771 0 5.657 0 6.828 1.172S22 6.229 22 10v1c0 2.828 0 4.243-.879 5.121C20.243 17 18.828 17 16 17H8c-2.828 0-4.243 0-5.121-.879C2 15.243 2 13.828 2 11z"/><path strokeLinecap="round" d="M16 22H8m4-5v5m10-9H2"/></g></svg>),
  'global': (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><g fill="none" stroke="currentColor" strokeWidth="1.5"><path d="M22 12a10 10 0 1 1-20.001 0A10 10 0 0 1 22 12Z"/><path d="M16 12c0 1.313-.104 2.614-.305 3.827c-.2 1.213-.495 2.315-.867 3.244c-.371.929-.812 1.665-1.297 2.168c-.486.502-1.006.761-1.531.761s-1.045-.259-1.53-.761c-.486-.503-.927-1.24-1.298-2.168c-.372-.929-.667-2.03-.868-3.244A23.6 23.6 0 0 1 8 12c0-1.313.103-2.614.304-3.827s.496-2.315.868-3.244c.371-.929.812-1.665 1.297-2.168C10.955 2.26 11.475 2 12 2s1.045.259 1.53.761c.486.503.927 1.24 1.298 2.168c.372.929.667 2.03.867 3.244C15.897 9.386 16 10.687 16 12Z"/><path strokeLinecap="round" d="M2 12h20"/></g></svg>),
  'widget': (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><path fill="none" stroke="currentColor" strokeWidth="1.5" d="M2.5 6.5c0-1.886 0-2.828.586-3.414S4.614 2.5 6.5 2.5s2.828 0 3.414.586s.586 1.528.586 3.414s0 2.828-.586 3.414s-1.528.586-3.414.586s-2.828 0-3.414-.586S2.5 8.386 2.5 6.5Zm11 11c0-1.886 0-2.828.586-3.414s1.528-.586 3.414-.586s2.828 0 3.414.586s.586 1.528.586 3.414s0 2.828-.586 3.414s-1.528.586-3.414.586s-2.828 0-3.414-.586s-.586-1.528-.586-3.414Zm-11 0c0-1.886 0-2.828.586-3.414S4.614 13.5 6.5 13.5s2.828 0 3.414.586s.586 1.528.586 3.414s0 2.828-.586 3.414s-1.528.586-3.414.586s-2.828 0-3.414-.586S2.5 19.386 2.5 17.5Zm11-11c0-1.886 0-2.828.586-3.414S15.614 2.5 17.5 2.5s2.828 0 3.414.586s.586 1.528.586 3.414s0 2.828-.586 3.414s-1.528.586-3.414.586s-2.828 0-3.414-.586S13.5 8.386 13.5 6.5Z"/></svg>),
  'key-minimalistic': (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><path fill="currentColor" d="m18.977 5.023l.53-.53zm0 9.767l.53.53zM7.146 12.668l-.53-.53zM3.433 16.38l.53.53zm4.187 4.187l-.53-.53zm3.712-3.713l-.53-.53zm-8.323.736l-.745.083zm.232 2.089l-.745.083zm1.08 1.08l-.083.745zm2.089.232l.082-.745zm-2.886-.723l.53-.53zm.208.208l-.53.53zm3.672-8.845l-.726.187zm4.965 4.965l-.187.726zm-4.73.467a.75.75 0 1 0-1.055 1.066zm5.477-6.18a1.25 1.25 0 0 1 0-1.767l-1.06-1.06a2.75 2.75 0 0 0 0 3.889zm1.768 0a1.25 1.25 0 0 1-1.768 0l-1.06 1.062a2.75 2.75 0 0 0 3.889 0zm0-1.767a1.25 1.25 0 0 1 0 1.768l1.06 1.06a2.75 2.75 0 0 0 0-3.889zm1.06-1.06a2.75 2.75 0 0 0-3.889 0l1.061 1.06a1.25 1.25 0 0 1 1.768 0zm2.503-2.503a6.157 6.157 0 0 1 0 8.707l1.06 1.06a7.657 7.657 0 0 0 0-10.827zm1.06-1.06a7.657 7.657 0 0 0-10.828 0l1.06 1.06a6.157 6.157 0 0 1 8.708 0zM6.615 12.138L2.903 15.85l1.06 1.06l3.714-3.71zm1.535 8.959l1.24-1.24l-1.06-1.061l-1.24 1.24zm1.24-1.24l2.472-2.472l-1.06-1.061l-2.472 2.472zm-7.126-2.184l.232 2.089l1.49-.166l-.232-2.088zm1.974 3.831l2.089.232l.165-1.49l-2.088-.232zm-1.244-.706l.208.208l1.06-1.06l-.208-.209zm1.41-.784a.24.24 0 0 1-.141-.068l-1.061 1.06c.279.28.644.455 1.036.498zm-1.908-.252c.043.392.219.757.498 1.036l1.06-1.06a.24.24 0 0 1-.067-.142zm4.593.274a.73.73 0 0 1-.597.21l-.165 1.49a2.23 2.23 0 0 0 1.823-.64zM2.903 15.85a2.23 2.23 0 0 0-.64 1.823l1.491-.165a.73.73 0 0 1 .21-.597zm5.228-4.405A6.15 6.15 0 0 1 9.74 5.553l-1.06-1.06a7.65 7.65 0 0 0-2.002 7.325zm10.316 2.815a6.15 6.15 0 0 1-5.892 1.61l-.373 1.452a7.65 7.65 0 0 0 7.325-2.001zm-6.585 3.124c.056-.055.17-.1.32-.062l.373-1.453c-.588-.15-1.27-.028-1.753.455zM7.676 13.2c.483-.483.606-1.166.455-1.754l-1.453.373c.038.15-.007.264-.063.32zm1.711 5.594l-1.749-1.73l-1.054 1.066l1.749 1.73z"/></svg>),
  'tuning': (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><g fill="none" stroke="currentColor" strokeWidth="1.5"><path d="M14 14.5a3 3 0 1 1 6 0a3 3 0 0 1-6 0Zm-10-5a3 3 0 1 0 6 0a3 3 0 0 0-6 0Z"/><path strokeLinecap="round" d="M16.959 9V2m-10 13v7m10 0v-2m-10-18v2"/></g></svg>),
  'bell': (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><g fill="none" stroke="currentColor" strokeWidth="1.5"><path d="M18.75 9.71v-.705C18.75 5.136 15.726 2 12 2S5.25 5.136 5.25 9.005v.705a4.4 4.4 0 0 1-.692 2.375L3.45 13.81c-1.011 1.574-.239 3.716 1.52 4.214a25.8 25.8 0 0 0 14.06 0c1.759-.498 2.531-2.64 1.52-4.214l-1.108-1.725a4.4 4.4 0 0 1-.692-2.375Z"/><path strokeLinecap="round" d="M7.5 19c.655 1.748 2.422 3 4.5 3s3.845-1.252 4.5-3"/></g></svg>),
  'keyboard': (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><g fill="none"><path fill="currentColor" d="M7 9a1 1 0 1 1-2 0a1 1 0 0 1 2 0m0 3a1 1 0 1 1-2 0a1 1 0 0 1 2 0m3 0a1 1 0 1 1-2 0a1 1 0 0 1 2 0m0-3a1 1 0 1 1-2 0a1 1 0 0 1 2 0m3 0a1 1 0 1 1-2 0a1 1 0 0 1 2 0m0 3a1 1 0 1 1-2 0a1 1 0 0 1 2 0m3-3a1 1 0 1 1-2 0a1 1 0 0 1 2 0m0 3a1 1 0 1 1-2 0a1 1 0 0 1 2 0m3-3a1 1 0 1 1-2 0a1 1 0 0 1 2 0m0 3a1 1 0 1 1-2 0a1 1 0 0 1 2 0"/><path stroke="currentColor" strokeWidth="1.5" d="M2 11c0-2.828 0-4.243.879-5.121C3.757 5 5.172 5 8 5h8c2.828 0 4.243 0 5.121.879C22 6.757 22 8.172 22 11v2c0 2.828 0 4.243-.879 5.121C20.243 19 18.828 19 16 19H8c-2.828 0-4.243 0-5.121-.879C2 17.243 2 15.828 2 13z"/><path stroke="currentColor" strokeLinecap="round" strokeWidth="1.5" d="M7 16h10"/></g></svg>),
  'archive-minimalistic': (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><g fill="none" stroke="currentColor" strokeWidth="1.5"><path d="M2 12c0-4.714 0-7.071 1.464-8.536C4.93 2 7.286 2 12 2s7.071 0 8.535 1.464C22 4.93 22 7.286 22 12"/><path d="M2 14c0-2.8 0-4.2.545-5.27A5 5 0 0 1 4.73 6.545C5.8 6 7.2 6 10 6h4c2.8 0 4.2 0 5.27.545a5 5 0 0 1 2.185 2.185C22 9.8 22 11.2 22 14s0 4.2-.545 5.27a5 5 0 0 1-2.185 2.185C18.2 22 16.8 22 14 22h-4c-2.8 0-4.2 0-5.27-.545a5 5 0 0 1-2.185-2.185C2 18.2 2 16.8 2 14Z"/><path strokeLinecap="round" strokeLinejoin="round" d="m9.5 14.4l1.429 1.6l3.571-4"/></g></svg>),
  'alt-arrow-left': (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><path fill="none" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.5" d="m15 5l-6 7l6 7"/></svg>),
  'volume-loud': (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><g fill="none" stroke="currentColor" strokeWidth="1.5"><path d="M2 12c0-1.886 0-2.828.586-3.414C3.172 8 4.114 8 6 8h.343a2 2 0 0 0 1.414-.586l2.829-2.828c1.26-1.26 1.89-1.521 2.414-1.086.523.434.5 1.267.5 2.5v12c0 1.233.023 2.066-.5 2.5-.524.435-1.154.174-2.414-1.086l-2.829-2.828A2 2 0 0 0 6.343 16H6c-1.886 0-2.828 0-3.414-.586C2 14.828 2 13.886 2 12Z"/><path strokeLinecap="round" d="M17 8.5c.63.897 1 1.99 1 3.17s-.37 2.273-1 3.17M19.5 6a9.46 9.46 0 0 1 2 5.83c0 2.19-.744 4.208-1.994 5.815"/></g></svg>),
  'archive-up-minimalistic': (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><g fill="none" stroke="currentColor" strokeWidth="1.5"><path d="M2 12c0-4.714 0-7.071 1.464-8.536C4.93 2 7.286 2 12 2s7.071 0 8.535 1.464C22 4.93 22 7.286 22 12"/><path d="M2 14c0-2.8 0-4.2.545-5.27A5 5 0 0 1 4.73 6.545C5.8 6 7.2 6 10 6h4c2.8 0 4.2 0 5.27.545a5 5 0 0 1 2.185 2.185C22 9.8 22 11.2 22 14s0 4.2-.545 5.27a5 5 0 0 1-2.185 2.185C18.2 22 16.8 22 14 22h-4c-2.8 0-4.2 0-5.27-.545a5 5 0 0 1-2.185-2.185C2 18.2 2 16.8 2 14Z"/><path strokeLinecap="round" strokeLinejoin="round" d="M12 16.5V10m0 0l-2.5 2.5M12 10l2.5 2.5"/></g></svg>),
  'trash-bin-minimalistic': (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><path fill="none" stroke="currentColor" strokeLinecap="round" strokeWidth="1.5" d="M9.17 4a3.001 3.001 0 0 1 5.66 0m5.67 2h-17m15.333 2.5l-.46 6.9c-.177 2.654-.265 3.981-1.13 4.79s-2.196.81-4.856.81h-.774c-2.66 0-3.991 0-4.856-.81c-.865-.809-.954-2.136-1.13-4.79l-.46-6.9M9.5 11l.5 5m4.5-5l-.5 5"/></svg>),
  'danger-triangle': (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><g fill="none"><path stroke="currentColor" strokeWidth="1.5" d="M5.312 10.762C8.23 5.587 9.689 3 12 3s3.77 2.587 6.688 7.762l.364.644c2.425 4.3 3.638 6.45 2.542 8.022S17.786 21 12.364 21h-.728c-5.422 0-8.134 0-9.23-1.572s.117-3.722 2.542-8.022z"/><path stroke="currentColor" strokeLinecap="round" strokeWidth="1.5" d="M12 8v5"/><circle cx="12" cy="16" r="1" fill="currentColor"/></g></svg>),
  'laptop': (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><g fill="none"><path stroke="currentColor" strokeLinecap="round" strokeWidth="1.5" d="M19.647 15.536H4.353m15.294 0V8c0-1.886 0-2.828-.586-3.414C18.476 4 17.533 4 15.647 4H8.353c-1.886 0-2.828 0-3.414.586S4.353 6.114 4.353 8v7.536m15.294 0l1.744 1.8l.088.092a2 2 0 0 1 .52 1.284l.001.127c0 .15 0 .224-.004.287a2 2 0 0 1-1.87 1.87a5 5 0 0 1-.287.004H4.161c-.15 0-.224 0-.287-.004a2 2 0 0 1-1.87-1.87C2 19.063 2 18.988 2 18.84l.001-.127a2 2 0 0 1 .52-1.284l.088-.092l1.744-1.8M9.5 18.5h5"/><path fill="currentColor" d="M12.75 6.75a.75.75 0 1 1-1.5 0a.75.75 0 0 1 1.5 0"/></g></svg>),
  'smartphone': (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><g fill="none" stroke="currentColor" strokeWidth="1.5"><path d="M5 9c0-3.3 0-4.95 1.025-5.975C7.05 2 8.7 2 12 2s4.95 0 5.975 1.025C19 4.05 19 5.7 19 9v6c0 3.3 0 4.95-1.025 5.975C16.95 22 15.3 22 12 22s-4.95 0-5.975-1.025C5 19.95 5 18.3 5 15z"/><path strokeLinecap="round" d="M15 19H9"/></g></svg>),
  'settings-minimalistic': (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><g fill="none" stroke="currentColor" strokeWidth="1.5"><path d="M7.843 3.802C9.872 2.601 10.886 2 12 2s2.128.6 4.157 1.802l.686.406c2.029 1.202 3.043 1.803 3.6 2.792c.557.99.557 2.19.557 4.594v.812c0 2.403 0 3.605-.557 4.594s-1.571 1.59-3.6 2.791l-.686.407C14.128 21.399 13.114 22 12 22s-2.128-.6-4.157-1.802l-.686-.407c-2.029-1.2-3.043-1.802-3.6-2.791C3 16.01 3 14.81 3 12.406v-.812C3 9.19 3 7.989 3.557 7s1.571-1.59 3.6-2.792z"/><circle cx="12" cy="12" r="3"/></g></svg>),
  'pen': (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><path fill="none" stroke="currentColor" strokeWidth="1.5" d="m14.36 4.079l.927-.927a3.932 3.932 0 0 1 5.561 5.561l-.927.927m-5.56-5.561s.115 1.97 1.853 3.707C17.952 9.524 19.92 9.64 19.92 9.64m-5.56-5.561l-8.522 8.52c-.577.578-.866.867-1.114 1.185a6.6 6.6 0 0 0-.749 1.211c-.173.364-.302.752-.56 1.526l-1.094 3.281m17.6-10.162L11.4 18.16c-.577.577-.866.866-1.184 1.114a6.6 6.6 0 0 1-1.211.749c-.364.173-.751.302-1.526.56l-3.281 1.094m0 0l-.802.268a1.06 1.06 0 0 1-1.342-1.342l.268-.802m1.876 1.876l-1.876-1.876"/></svg>),
  'plus': (<svg viewBox="0 0 16 16" fill="none" className="size-full" aria-hidden="true"><path d="M8 3.25v9.5M3.25 8h9.5" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round"/></svg>),
  'check': (<svg viewBox="0 0 16 16" fill="none" className="size-full" aria-hidden="true"><path d="M3.5 8.5l3 3 6-7" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round"/></svg>),
  'refresh': (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><g fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"><path d="M19.73 11a8 8 0 1 1-2.07-4.66"/><path strokeLinejoin="round" d="M17.16 3.34l.63 2.86l-2.86.63"/></g></svg>),
  'close': (<svg viewBox="0 0 16 16" fill="none" className="size-full" aria-hidden="true"><path d="m4.75 4.75 6.5 6.5m0-6.5-6.5 6.5" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round"/></svg>),
  'hard-drive': (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><g fill="none" stroke="currentColor" strokeWidth="1.5"><path d="M2.75 12c0-1.886 0-2.828.586-3.414C3.922 8 4.864 8 6.75 8h10.5c1.886 0 2.828 0 3.414.586c.586.586.586 1.528.586 3.414s0 2.828-.586 3.414c-.586.586-1.528.586-3.414.586H6.75c-1.886 0-2.828 0-3.414-.586C2.75 14.828 2.75 13.886 2.75 12Z"/><path strokeLinecap="round" d="M6.25 12h.01M9.5 12h.01"/><path strokeLinecap="round" d="M18 12h-4"/></g></svg>),
};
export type SettingsIcon = keyof typeof icons;
/// Rust: crate::icons::icon(path) - the same SVG assets, same currentColor paint.
export function settingsIcon(name: SettingsIcon) { return icons[name]; }

/// Rust: page_column(). Centered page column, `mx-auto w-full max-w-3xl px-6 pb-16 pt-8`.
export function pageColumn(children: ReactNode) {
  return <div className="w-full max-w-184 mx-auto px-6 pt-8 pb-16 flex flex-col">{children}</div>;
}

/// Rust: page_header(theme, title, count). Title and optional count on one baseline.
export function pageHeader(title: string, count?: number) {
  return (
    <div className="flex flex-row items-baseline gap-2.5">
      <h1 className="text-ui-16 font-semibold text-text">{title}</h1>
      {count !== undefined && <span className="text-ui-13 text-text-faint">{count}</span>}
    </div>
  );
}

/// Rust: page_subtitle(theme, copy). `narrow` is the Notifications page's
/// max_w(512) + line_height(20) on the same helper.
export function pageSubtitle(copy: string, narrow = false) {
  return <p className={narrow
    ? 'mt-1 text-ui-13 text-text-muted max-w-96 leading-normal'
    : 'mt-1 text-ui-13 text-text-muted'}>{copy}</p>;
}

/// Rust: field_label(theme, label). The caption over a picker, not a page headline.
export function fieldLabel(label: string) {
  return <div className="text-ui-13 font-medium text-text">{label}</div>;
}

/// Rust: option_card_row(). A row of equally sized preview cards.
export function optionCardRow(children: ReactNode) {
  return <div className="flex flex-row items-start gap-4 w-full">{children}</div>;
}

/// Rust: option_card(theme, label, selected, preview). The preview frame is the
/// control; the Rust caller adds `.id(..)` and `.on_click(..)`, so `onSelect` is
/// the callback out. The preview must round its own corners to `rounded-md`.
export function optionCard(label: string, selected: boolean, preview: ReactNode, onSelect?: () => void) {
  return (
    <button type="button" onClick={onSelect} aria-pressed={selected} data-option={label}
      className="flex-1 min-w-0 flex flex-col items-center gap-2 cursor-pointer">
      <span className={selected
        ? 'h-40 w-full rounded-md overflow-hidden border border-accent flex'
        : 'h-40 w-full rounded-md overflow-hidden border border-border flex'}>{preview}</span>
      <span className={selected
        ? 'text-ui-13 font-medium text-accent'
        : 'text-ui-13 font-normal text-text-muted'}>{label}</span>
    </button>
  );
}

/// Rust: section_card(theme). The card tone over glass.
export function sectionCard(children: ReactNode) {
  return <div className="mt-6 rounded-xl border border-border bg-surface-card overflow-hidden flex flex-col">{children}</div>;
}

/// Rust: card_row(theme, first). `dimmed` is the Notifications sub-option's
/// `.opacity(0.55)` while its parent toggle is off.
export function cardRow(first: boolean, children: ReactNode, dimmed = false) {
  if (dimmed) {
    return <div className={first
      ? 'px-5 py-3.5 flex flex-row items-center gap-3.5 hover:bg-wash/5 opacity-55'
      : 'px-5 py-3.5 flex flex-row items-center gap-3.5 hover:bg-wash/5 opacity-55 border-t border-border'}>{children}</div>;
  }
  return <div className={first
    ? 'px-5 py-3.5 flex flex-row items-center gap-3.5 hover:bg-wash/5'
    : 'px-5 py-3.5 flex flex-row items-center gap-3.5 hover:bg-wash/5 border-t border-border'}>{children}</div>;
}

/// Rust: row_tile(theme, icon_path). The identity tile around a 16px icon.
export function rowTile(iconPath: SettingsIcon) {
  return (
    <div className="flex-none size-9 rounded-lg border border-border bg-wash/5 flex items-center justify-center">
      <span className="size-4 flex text-text-muted">{settingsIcon(iconPath)}</span>
    </div>
  );
}

/// Rust: row_title(theme, title). ROW_TITLE_SIZE is 13.
export function rowTitle(title: string) {
  return <div className="min-w-0 truncate text-ui-13 font-medium text-text">{title}</div>;
}

/// Rust: meta_line(theme, fragments). Fragments joined by a quieter dot.
export function metaLine(fragments: readonly ReactNode[]) {
  return (
    <div className="mt-0.25 flex flex-row flex-wrap items-center gap-x-2 gap-y-0.5 text-ui-12 text-text-faint">
      {fragments.flatMap((fragment, index) => index === 0
        ? [<span key={`fragment-${index}`}>{fragment}</span>]
        : [<span key={`dot-${index}`} className="text-text-faint/50">{'·'}</span>,
          <span key={`fragment-${index}`}>{fragment}</span>])}
    </div>
  );
}

/// Rust: badge(theme, label). The right-anchored outline pill.
export function badge(label: string) {
  return <span className="flex-none px-2 py-0.5 rounded-full border border-border text-ui-10 text-text-muted">{label}</span>;
}

/// Rust: badge_active(theme, label). The emerald status pill.
export function badgeActive(label: string) {
  return <span className="flex-none px-2 py-0.5 rounded-full bg-success/14 text-ui-10 text-success-muted/88">{label}</span>;
}

/// Rust: toggle_switch(theme, on). The Rust helper is display-only and the
/// caller adds `.id(..)` and `.on_click(..)`; here the page owns the boolean and
/// passes `onToggle`. Hover, active and focus are sandbox additions - the native
/// pill has none - so a designer can feel the control.
export function toggleSwitch(on: boolean, label: string, onToggle?: () => void, disabled = false) {
  return (
    <button type="button" role="switch" aria-checked={on} aria-label={label} disabled={disabled}
      onClick={onToggle} data-toggle={label}
      className={on
        ? 'flex-none w-8 h-5 rounded-full relative cursor-pointer bg-text hover:bg-text/88 active:bg-text/50 focus:bg-text/88'
        : 'flex-none w-8 h-5 rounded-full relative cursor-pointer bg-wash/10 hover:bg-wash/14 active:bg-wash/50 focus:bg-wash/14'}>
      <span className={on
        ? 'absolute top-0.5 left-3.5 size-4 rounded-full bg-on-solid'
        : 'absolute top-0.5 left-0.5 size-4 rounded-full bg-wash/88'} />
    </button>
  );
}

/// Rust: ghost_action(theme) plus ghost_hover(theme, s), which in Rust is a
/// StyleRefinement the caller applies. Here it is this button's hover, active
/// and focus class list. `muted` is the Servers Remove button's resting
/// `.opacity(0.7)` that its own hover lifts.
export function ghostAction(label: string, iconPath: SettingsIcon, onClick?: () => void, muted = false) {
  return (
    <button type="button" onClick={onClick} data-action={label}
      className={muted
        ? 'flex-none flex flex-row items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-ui-12 text-text-muted cursor-pointer opacity-55 hover:opacity-100 hover:bg-wash/5 hover:text-text active:bg-wash/10 focus:opacity-100 focus:bg-wash/5 focus:text-text'
        : 'flex-none flex flex-row items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-ui-12 text-text-muted cursor-pointer hover:bg-wash/5 hover:text-text active:bg-wash/10 focus:bg-wash/5 focus:text-text'}>
      <span className="flex-none size-3.5 flex">{settingsIcon(iconPath)}</span>
      {label}
    </button>
  );
}

/// Rust: error_strip(theme, message). Dismissible where the page passes onDismiss.
export function errorStrip(message: string, onDismiss?: () => void) {
  const body = (
    <>
      <span className="flex-none mt-0.5 size-4 flex">{settingsIcon('danger-triangle')}</span>
      <span className="min-w-0">{message}</span>
    </>
  );
  if (!onDismiss) return <div role="alert" className="mt-4 px-4 py-3 rounded-xl border border-danger/14 bg-danger/5 text-ui-12 text-danger-muted/88 flex flex-row items-start gap-2">{body}</div>;
  return (
    <button type="button" onClick={onDismiss} aria-label="Dismiss error"
      className="mt-4 px-4 py-3 rounded-xl border border-danger/14 bg-danger/5 text-ui-12 text-danger-muted/88 flex flex-row items-start gap-2 w-full text-left cursor-pointer hover:bg-danger/10 active:bg-danger/14 focus:bg-danger/10">{body}</button>
  );
}

/// Rust: warning_strip(theme, message). The amber sibling of error_strip.
export function warningStrip(message: string) {
  return (
    <div role="alert" className="mt-2 px-4 py-2.5 rounded-xl border border-warning/14 bg-warning/5 text-ui-12 text-warning-muted/88 flex flex-row items-start gap-2">
      <span className="flex-none mt-0.5 size-3.5 flex">{settingsIcon('danger-triangle')}</span>
      <span className="min-w-0">{message}</span>
    </div>
  );
}
