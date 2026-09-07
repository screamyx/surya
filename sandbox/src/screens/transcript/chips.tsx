// Rust: app/crates/ui/src/transcript.rs (error_chip, input_chip,
// permission_chip).
import { renderIcon } from '../../icons';

/// Rust: `error_chip`. A subtle red-tinted wash, never a bare red-stroke box:
/// a 20px red tile holding the danger triangle, a medium "Error" label, then
/// the message. The message WRAPS instead of truncating - a startup-crash
/// error carries the agent's exit status and stderr, and a one-line ellipsis
/// is what made zeronsh/comet#95 undiagnosable from the screenshot.
export function errorChip(message: string) {
  return (
    <div className="py-1 w-full">
      <div className="w-full flex items-center gap-2 overflow-hidden rounded-lg border border-danger/14 bg-danger/5 px-2 py-1.5 text-ui-12">
        <div className="flex-none size-5 rounded-md bg-danger/14 flex items-center justify-center text-danger-muted/88">
          <span className="size-3 flex items-center justify-center">{renderIcon('danger-triangle')}</span>
        </div>
        <div className="flex-none font-medium text-danger-muted/88">Error</div>
        <div className="min-w-0 flex-1 text-text/88">{message}</div>
      </div>
    </div>
  );
}

/// Rust: `input_chip`. A passive one-line chip marking a question the agent
/// asked - the interactive controls live in the composer. Neutral tones
/// throughout; resolution never recolors the chip.
export function inputChip(header: string, resolved: boolean) {
  const value = resolved ? header : 'Awaiting your answer…';
  return (
    <div className="py-1 w-full">
      <div className="h-8 w-full flex items-center gap-2 overflow-hidden rounded-lg border border-wash/10 bg-wash/5 px-2 text-ui-12">
        <div className="flex-none size-5 rounded-md bg-wash/10 flex items-center justify-center text-text-muted">
          <span className="size-3 flex items-center justify-center">{renderIcon('chat-round-line')}</span>
        </div>
        <div className="flex-none font-medium text-text-muted">Question</div>
        <div className="min-w-0 flex-1 truncate whitespace-nowrap text-text/88">{value}</div>
      </div>
    </div>
  );
}

/// Rust: `permission_chip`, the twin of `input_chip`. Unresolved it says what
/// a question says - something is waiting on you. Resolved it states the
/// answer in a word, because a tool that did not run leaves no other trace and
/// "nothing happened" is not something the user should have to infer.
export function permissionChip(
  toolName: string,
  command: string,
  resolved: boolean,
  allowed: boolean,
  always: boolean,
) {
  let value = 'Awaiting your answer…';
  if (resolved && allowed && always) value = `Always allowed - ${command}`;
  else if (resolved && allowed) value = `Allowed - ${command}`;
  else if (resolved) value = `Denied - ${command}`;
  return (
    <div className="py-1 w-full">
      <div className="h-8 w-full flex items-center gap-2 overflow-hidden rounded-lg border border-wash/10 bg-wash/5 px-2 text-ui-12">
        <div className="flex-none size-5 rounded-md bg-wash/10 flex items-center justify-center text-text-muted">
          <span className="size-3 flex items-center justify-center">{renderIcon('key-minimalistic')}</span>
        </div>
        <div className="flex-none font-medium text-text-muted">{toolName}</div>
        <div className="min-w-0 flex-1 truncate whitespace-nowrap text-text/88">{value}</div>
      </div>
    </div>
  );
}
