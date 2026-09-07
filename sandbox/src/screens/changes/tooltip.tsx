// Rust: app/crates/ui/src/changes.rs, struct DiffHeaderTooltip and its Render
// impl. Split out of Changes.tsx so the header controls can anchor it to the
// control that owns it, the way `.tooltip(...)` does on the native element.
export function DiffHeaderTooltip({ label }: { label: string }) {
  return (
    <div role="tooltip" className="px-2 py-1.5 rounded-md border border-border-strong bg-surface-raised text-ui-11 text-text whitespace-nowrap">{label}</div>
  );
}
