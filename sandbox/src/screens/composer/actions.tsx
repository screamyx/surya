// Rust: app/crates/ui/src/composer.rs, Composer::render_send_button (6254) and
// the `composer-attach` paperclip built inline in Composer::render (6655).
import type { SendMode } from '../../fixtures/composer';
import { renderIcon } from '../../icons';

// render_send_button: a 28px filled circle - up arrow to send or steer, a dark
// rounded square on the same light circle to stop.
export function renderSendButton(mode: SendMode, blocked: boolean, onSend: () => void, onStop: () => void) {
  if (mode === 'stop') {
    return (
      <button type="button" data-action="composer-stop" aria-label="Stop the run" onClick={onStop}
        className="size-7 flex-none flex items-center justify-center rounded-full bg-text cursor-pointer hover:bg-text/88 active:bg-text/50 focus:bg-text/88">
        <span className="size-3 rounded-sm bg-bg" />
      </button>
    );
  }
  const label = mode === 'steer' ? 'Steer the current run' : 'Send';
  if (blocked) {
    return (
      <button type="button" data-action="composer-send" aria-label={label} aria-disabled="true"
        className="size-7 flex-none flex items-center justify-center rounded-full bg-text opacity-50">
        <span className="size-3.5 text-bg">{renderIcon('arrow-up')}</span>
      </button>
    );
  }
  return (
    <button type="button" data-action="composer-send" aria-label={label} onClick={onSend}
      className="size-7 flex-none flex items-center justify-center rounded-full bg-text cursor-pointer hover:bg-text/88 active:bg-text/50 focus:bg-text/88">
      <span className="size-3.5 text-bg">{renderIcon('arrow-up')}</span>
    </button>
  );
}

// The paperclip: opens the native image picker. Paste and drop feed the same
// strip. The parent cluster owns the spacing.
export function renderAttachButton(onAttach: () => void) {
  return (
    <button type="button" data-action="composer-attach" aria-label="Attach" onClick={onAttach}
      className="size-7 flex-none flex items-center justify-center rounded-full cursor-pointer motion-hover-fade hover:bg-element-hover active:bg-element-active focus:bg-element-hover">
      <span className="size-4 text-text-muted">{renderIcon('paperclip')}</span>
    </button>
  );
}
