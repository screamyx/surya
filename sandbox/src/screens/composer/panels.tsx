// Rust: app/crates/ui/src/composer.rs, Composer::render_permission (5901) and
// Composer::render_wizard (6051); permission_options.rs::ROWS for the answer
// lines and their order.
import type { ReactNode } from 'react';
import type { PermissionRequest, WizardModel } from '../../fixtures/composer';
import { trackedUpper, btnGhost, btnPrimary } from '../popover';

// The panel chrome both sheets wear: the composer pill's own shape.
function panel(label: string, children: ReactNode) {
  return (
    <div aria-label={label}
      className="flex flex-col rounded-xl border border-border bg-input-bg motion-fade-quick">
      {children}
    </div>
  );
}

// permission_options.rs::ROWS, in the order they are drawn and numbered. The
// stop-asking line is LAST on purpose: 1, 2 and 3 are muscle memory.
export const PERMISSION_ROWS: readonly string[] = [
  'Allow',
  'Always allow exactly this in this project',
  'Deny',
  'Allow, and stop asking in this chat',
  'Always allow exactly this everywhere',
];

function optionRow(
  label: string,
  index: number,
  picked: boolean,
  key: string,
  onPick: () => void,
) {
  return (
    <button key={key} type="button" data-option={key} onClick={onPick}
      className={picked
        ? 'flex flex-row items-center gap-3 px-3.5 py-2.5 rounded-xl border border-wash/14 bg-wash/10 text-left cursor-pointer'
        : 'flex flex-row items-center gap-3 px-3.5 py-2.5 rounded-xl border border-wash/5 bg-wash/5 text-left cursor-pointer motion-hover-fade hover:bg-element-hover active:bg-element-active focus:bg-element-hover'}>
      <span className={picked
        ? 'flex-1 min-w-0 text-ui-13 font-medium text-text'
        : 'flex-1 min-w-0 text-ui-13 font-medium text-text/88'}>{label}</span>
      {index < 9 && (
        <span className={picked
          ? 'flex-none size-5 flex items-center justify-center rounded-md bg-wash/14 text-ui-11 text-text'
          : 'flex-none size-5 flex items-center justify-center rounded-md bg-wash/5 text-ui-11 text-text-muted/50'}>
          {index + 1}
        </span>
      )}
    </button>
  );
}

// render_permission: a blocked tool takes the composer's place the way a
// question does. Escape has no row of its own, so it is said out loud.
export function renderPermission(
  request: PermissionRequest,
  onAnswer: (requestId: string, answer: string) => void,
) {
  return panel('Permission', (
    <>
      <div className="px-4 pt-4 flex flex-col gap-1.5">
        <span className="text-ui-10 font-medium text-text-muted/50">{trackedUpper('Permission')}</span>
        <span className="text-ui-16 font-medium text-text">Allow {request.toolName}?</span>
        {request.command.trim() !== '' && (
          <span className="min-w-0 px-2 py-1.5 rounded-lg bg-wash/5 text-ui-12 text-text">{request.command}</span>
        )}
      </div>
      <div className="px-4 py-2.5 flex flex-col gap-1">
        {PERMISSION_ROWS.map((label, index) => optionRow(
          label, index, false, `permission-option-${index}`,
          () => onAnswer(request.requestId, label),
        ))}
      </div>
      <div className="px-4 pb-3.5 text-ui-11 text-text-muted/50">Esc denies</div>
    </>
  ));
}

// render_wizard: the agent-asked-a-question sheet. Uppercase header, a page
// counter chip, option rows with number chips, a free-text override over a
// hairline, and Back / Next-Submit.
export function renderWizard(
  wizard: WizardModel,
  picks: readonly number[],
  input: ReactNode,
  onSelect: (index: number) => void,
  onBack: () => void,
  onAdvance: () => void,
) {
  const question = wizard.questions[wizard.page];
  if (question === undefined) return null;
  const last = wizard.page + 1 >= wizard.questions.length;
  const canAdvance = picks.length > 0;
  return panel('Question', (
    <>
      <div className="px-4 pt-4 flex flex-col">
        <div className="flex flex-row items-center gap-2.5">
          <span className="text-ui-10 font-medium text-text-muted/50">{trackedUpper(question.header)}</span>
          {wizard.questions.length > 1 && (
            <span className="h-5 px-1.5 flex items-center rounded-md bg-wash/5 text-ui-10 font-medium text-text-muted/50">
              {wizard.page + 1}/{wizard.questions.length}
            </span>
          )}
        </div>
        <div className="mt-1.5 text-ui-16 font-medium text-text">{question.question}</div>
        {question.multiSelect && (
          <div className="mt-1 text-ui-12 text-text-muted/88">Select one or more options.</div>
        )}
        <div className="mt-3 flex flex-col gap-1">
          {question.options.map((label, index) => optionRow(
            label, index, picks.includes(index), `wizard-option-${index}`,
            () => onSelect(index),
          ))}
        </div>
        <div className="mt-3 pt-3 pb-1 px-1 border-t border-wash/5">{input}</div>
      </div>
      <div className="flex flex-row justify-between items-center px-4 pb-4 pt-1">
        {wizard.page > 0 ? btnGhost('Back', onBack) : <span />}
        {btnPrimary(last ? 'Submit' : 'Next', onAdvance, !canAdvance)}
      </div>
    </>
  ));
}
