// Rust: app/crates/ui/src/transcript.rs (render_working_trailer, retry_send's
// affordance, FLAVOUR_WORDS, format_elapsed).
import { gradientSpinner } from './loaders';

/// Rust: what `render_working_trailer` derives from the session row. `word` is
/// the rotating flavour word (FLAVOUR_WORDS, one step every 7s), or "Sending"
/// during the send-to-turn bridge, or the queued line. `elapsed` is
/// `format_elapsed`, hidden while sending because the timer starts with the
/// turn, not the send.
export type WorkingTrailer = {
  word: string;
  elapsed: string;
  sending: boolean;
  queued: boolean;
};

/// Rust: the failed-send arm - past the grace window the trailer IS the retry
/// affordance, whatever the indicator fell back to.
export function undeliveredRetry(onRetrySend: () => void) {
  return (
    <button type="button" data-undelivered="1" onClick={onRetrySend}
      className="flex flex-row items-center gap-2 pt-4 cursor-pointer text-ui-12 text-danger hover:text-danger-strong active:text-danger-strong focus:text-danger-strong">
      Not delivered — click to retry
    </button>
  );
}

/// Rust: `render_working_trailer`. The loader rides INSIDE the conversation
/// flow, under the last row, so it reads as part of the streaming reply and
/// scrolls away with it. The queued line is warning-toned and drops the timer
/// with the word, because a durable local write waiting on connectivity is not
/// progress to count.
export function workingTrailer(trailer: WorkingTrailer) {
  return (
    <div data-working="1" className="flex flex-row items-center gap-2 pt-4 text-ui-11">
      {gradientSpinner()}
      <div className={trailer.queued ? 'text-ui-12 text-warning' : 'text-ui-12 text-text-muted'}>
        {trailer.queued ? trailer.word : `${trailer.word}…`}
      </div>
      {!trailer.sending && <div className="text-text-faint">{trailer.elapsed}</div>}
    </div>
  );
}
