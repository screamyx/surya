// Rust: app/crates/ui/src/transcript/stopped.rs (STOPPED_LABEL, mark).

/// Rust: `STOPPED_LABEL`. One word, the same word decision 17 uses for the
/// agent state, so the transcript and the agent tree agree.
export const STOPPED_LABEL = 'Stopped';

/// Rust: `mark`. A quiet label in the footer strip, beside the timestamp, and
/// deliberately NOT inside the hover fade the timestamp and copy button live
/// in: the point is that you can come back later and see why the answer is
/// short, and a cue you have to hunt for with the pointer is not that.
export function stoppedMark() {
  return (
    <div data-stopped="1" className="flex-none px-1.5 rounded-sm text-ui-11 bg-warning/14 text-warning">
      {STOPPED_LABEL}
    </div>
  );
}
