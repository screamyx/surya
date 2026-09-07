// Rust: app/crates/ui/src/change_requests.rs (ChangeRequestBadgeModel,
// ChangeRequestBadgeTone, ChangeRequestTooltip, pull_request_badge).

export type ChangeRequestState = 'open' | 'merged' | 'closed';
export type ChangeRequestSummary = { number: number; state: ChangeRequestState; title: string; url: string };

export type ChangeRequestBadgeModel = {
  number: string;
  stateLabel: string;
  title: string;
  tone: ChangeRequestState;
};

/// Rust: `ChangeRequestBadgeModel::from_summary`.
export function badgeModel(summary: ChangeRequestSummary): ChangeRequestBadgeModel {
  const stateLabel = summary.state === 'open' ? 'Open' : summary.state === 'merged' ? 'Merged' : 'Closed';
  return {
    number: `#${summary.number}`,
    stateLabel,
    title: summary.title.replace(/[\r\n]/g, ' '),
    tone: summary.state,
  };
}

/// Rust: `impl Render for ChangeRequestTooltip`. Tone maps open to success,
/// merged to code_text and closed to danger.
export function ChangeRequestTooltip({ summary }: { summary: ChangeRequestSummary }) {
  const model = badgeModel(summary);
  return (
    <div role="tooltip" className="max-w-80 px-2 py-1.5 flex flex-col gap-0.75 rounded-md border border-border-strong bg-surface-raised">
      <div className={model.tone === 'open' ? 'text-ui-11 font-medium text-success'
        : model.tone === 'merged' ? 'text-ui-11 font-medium text-code-text'
          : 'text-ui-11 font-medium text-danger'}>{`PR ${model.number} · ${model.stateLabel}`}</div>
      <div className="min-w-0 truncate whitespace-nowrap text-ui-11 text-text-muted">{model.title}</div>
    </div>
  );
}

/// Rust: `pull_request_badge`, the Sidebar surface. The Composer surface is
/// the same element at 20px with wider padding.
export function pullRequestBadge(summary: ChangeRequestSummary, hovered: boolean,
  onHover: (shown: boolean) => void, onOpenChangeRequest: (url: string) => void) {
  const model = badgeModel(summary);
  return (
    <div className="relative flex-none">
      <button type="button" data-change-request={model.number}
        onClick={() => onOpenChangeRequest(summary.url)}
        onMouseEnter={() => onHover(true)} onMouseLeave={() => onHover(false)}
        onFocus={() => onHover(true)} onBlur={() => onHover(false)}
        className={model.tone === 'open'
          ? 'h-4 flex-none flex flex-row items-center px-1 rounded-sm bg-success/10 text-ui-10 font-medium text-success/88 cursor-pointer hover:bg-success/14 hover:text-success active:bg-success/14 focus:bg-success/14'
          : model.tone === 'merged'
            ? 'h-4 flex-none flex flex-row items-center px-1 rounded-sm bg-code-text/10 text-ui-10 font-medium text-code-text/88 cursor-pointer hover:bg-code-text/14 hover:text-code-text active:bg-code-text/14 focus:bg-code-text/14'
            : 'h-4 flex-none flex flex-row items-center px-1 rounded-sm bg-danger/10 text-ui-10 font-medium text-danger/88 cursor-pointer hover:bg-danger/14 hover:text-danger active:bg-danger/14 focus:bg-danger/14'}>
        {model.number}
      </button>
      {hovered && <div className="absolute top-5 left-0"><ChangeRequestTooltip summary={summary} /></div>}
    </div>
  );
}
