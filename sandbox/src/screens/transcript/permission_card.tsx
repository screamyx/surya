// Rust: app/crates/ui/src/permission_options.rs (PermissionAnswer, ROWS,
// for_digit) and app/crates/ui/src/composer.rs (Composer::render_permission,
// which is where the card is hosted natively - in the composer's place, not
// inside the transcript's row list).

/// Rust: `PermissionAnswer`. The labels are copied exactly, not paraphrased:
/// the rule the engine writes is pinned to THIS one command, so "Always allow
/// in this project" promised a project-wide grant the rule never gave
/// (E2E-PERM-02), and the card and the Needs you inbox have to name the same
/// two reaches with the same two words (PERM-03).
export type PermissionAnswer =
  | 'allow'
  | 'allowAlways'
  | 'deny'
  | 'allowAndStopAsking'
  | 'allowAlwaysEverywhere';

/// Rust: `PermissionAnswer::label`.
export function answerLabel(answer: PermissionAnswer) {
  switch (answer) {
    case 'allow': return 'Allow';
    case 'allowAlways': return 'Always allow exactly this in this project';
    case 'allowAlwaysEverywhere': return 'Always allow exactly this everywhere';
    case 'deny': return 'Deny';
    case 'allowAndStopAsking': return 'Allow, and stop asking in this chat';
  }
}

/// Rust: `ROWS`, in the order they are drawn and numbered. The stop-asking
/// line is LAST rather than grouped with the other allows on purpose: 1, 2
/// and 3 are muscle memory, and a card is answered by typing a digit, so
/// renumbering them would turn a mis-key into "this chat runs tools without
/// asking from now on".
export const PERMISSION_ROWS: readonly PermissionAnswer[] = [
  'allow',
  'allowAlways',
  'deny',
  'allowAndStopAsking',
  'allowAlwaysEverywhere',
];

/// Rust: the `request_id`, `tool_name` and `command` `render_permission` takes.
export type PermissionAsk = { requestId: string; toolName: string; command: string };

/// Rust: `crate::popover::tracked_upper` - uppercase with a hair space
/// between letters, which is how the header gets its tracking without a
/// letter-spacing property.
function trackedUpper(label: string) {
  return label.toUpperCase().split('').join(' ');
}

/// Rust: `Composer::render_permission`. Deliberately the same container,
/// header and option rows as the wizard: a permission IS a question, and the
/// user should not have to learn a second shape for "something is waiting on
/// you". The number chip is the only place the keys are written down, so the
/// panel says how to answer it without a legend, and Escape is said out loud
/// because it has no row of its own.
export function PermissionCard(props: {
  ask: PermissionAsk;
  picked: PermissionAnswer | null;
  onAnswerPermission: (requestId: string, answer: PermissionAnswer) => void;
}) {
  const { ask, picked, onAnswerPermission } = props;
  return (
    <div data-permission-panel={ask.requestId}
      className="rounded-xl border border-border bg-surface-overlay flex flex-col">
      <div className="px-4 pt-4 flex flex-col gap-1.5">
        <div className="text-ui-10 font-medium text-text-muted">{trackedUpper('Permission')}</div>
        <div className="text-ui-16 font-medium text-text">{`Allow ${ask.toolName}?`}</div>
        {ask.command.trim() !== '' && (
          <div className="min-w-0 px-2 py-1.5 rounded-lg bg-wash/5 text-ui-12 text-text">{ask.command}</div>
        )}
      </div>
      <div className="px-4 py-2.5 flex flex-col gap-1">
        {PERMISSION_ROWS.map((answer, ix) => (
          <button key={answer} type="button" data-permission-option={answer}
            aria-pressed={picked === answer}
            onClick={() => onAnswerPermission(ask.requestId, answer)}
            className={picked === answer
              ? 'flex flex-row items-center gap-3 px-3.5 py-2.5 rounded-xl border border-wash/14 text-left cursor-pointer bg-wash/10 hover:bg-wash/14 active:bg-element-active focus:bg-wash/14'
              : 'flex flex-row items-center gap-3 px-3.5 py-2.5 rounded-xl border border-wash/5 text-left cursor-pointer bg-wash/5 hover:bg-wash/10 active:bg-element-active focus:bg-wash/10'}>
            <div className="flex-1 min-w-0 text-ui-13 font-medium text-text/88">{answerLabel(answer)}</div>
            <div className="flex-none size-5 flex items-center justify-center rounded-md bg-wash/5 text-ui-11 text-text-muted">
              {ix + 1}
            </div>
          </button>
        ))}
      </div>
      <div className="px-4 pb-3.5 text-ui-11 text-text-muted">Esc denies</div>
    </div>
  );
}
