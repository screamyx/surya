// Rust: app/crates/ui/src/inbox/chrome.rs (badge, hint_chip).
export function badge(label: string, tone: 'warning' | 'accent') {
  return <span className={tone === 'warning'
    ? 'flex-none px-1.5 py-0.25 rounded-full bg-warning/14 text-ui-10 font-medium text-warning'
    : 'flex-none px-1.5 py-0.25 rounded-full bg-accent/14 text-ui-10 font-medium text-accent'}>{label}</span>;
}
export function hintChip(label: string) {
  return <span className="flex-none px-1.5 py-1 rounded-md text-ui-11 text-text-faint">{label}</span>;
}
