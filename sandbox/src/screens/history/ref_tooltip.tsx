// Rust: app/crates/ui/src/history.rs (HistoryRefTooltip::render, line 292).
export type HistoryRefTooltipProps = { descriptions: readonly string[] };
export function HistoryRefTooltip({ descriptions }: HistoryRefTooltipProps) {
  return (
    <div role="tooltip" className="min-w-16 max-w-96 px-2 py-1.5 flex flex-col gap-0.75 rounded-md border border-border-strong bg-surface-raised text-ui-11 text-text-muted">
      {descriptions.map(description => (
        <div key={description} className="min-w-0 truncate whitespace-nowrap">{description}</div>
      ))}
    </div>
  );
}
