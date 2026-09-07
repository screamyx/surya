// Rust: app/crates/ui/src/transcript/retry.rs (RETRY_LABEL, control).

/// Rust: `RETRY_LABEL`. Decision 17 names the action "Retry".
export const RETRY_LABEL = 'Retry';

/// Rust: `control`. A quiet text button in the footer strip, outside the hover
/// fade: Retry is the way to run a stopped turn again, not a cue to hunt for.
/// Retry re-sends the last user prompt as a NEW turn; it does not resume the
/// interrupted one, which stays in the transcript as history. The send itself
/// reaches the engine, so it leaves here as a callback.
export function retryControl(rowId: string, prompt: string, onRetryTurn: (prompt: string) => void) {
  return (
    <button type="button" data-retry={rowId} onClick={() => onRetryTurn(prompt)}
      className="flex-none px-1.5 rounded-sm cursor-pointer text-ui-11 text-text-muted hover:bg-wash/10 hover:text-text active:bg-element-active focus:bg-wash/10">
      {RETRY_LABEL}
    </button>
  );
}
