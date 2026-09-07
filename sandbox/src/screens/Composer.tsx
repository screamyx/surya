// Rust: app/crates/ui/src/composer.rs, Composer (3423) and its Render impl
// (6329). Split helpers mirror the Rust boundaries: composer/input.tsx is
// ComposerInput, composer/actions.tsx the send and attach buttons,
// composer/attachments.tsx the staged strip and comments chip,
// composer/pickers.tsx the chip row and the two toolbars, composer/popups.tsx
// the @ and / completions, composer/panels.tsx the permission and question
// sheets.
import { useState } from 'react';
import type { ComposerModel, PickerKind } from '../fixtures/composer';
import { ComposerInput } from './composer/input';
import { renderSendButton, renderAttachButton } from './composer/actions';
import { renderAttachmentStrip, renderCommentsChip, renderLightbox } from './composer/attachments';
import { renderYoloChip, renderModelChip, renderTargetSelectors, renderFooter } from './composer/pickers';
import { renderHarnessModelPopover } from './composer/model_popover';
import { renderFileMentionPopup, renderSlashPopup } from './composer/popups';
import { renderPermission, renderWizard } from './composer/panels';
import { renderIcon } from '../icons';
import { MentionPathTooltip } from './composer/mention_tooltip';

export type ComposerProps = {
  model: ComposerModel;
  onEvent: (action: string) => void;
};

// Composer::render, the failure Notice: a subtle tinted wash, not a bare red
// stroke. Amber when the engine is not connected, red for send/run failures.
// Click dismisses.
function renderFailure(message: string, offline: boolean, onDismiss: () => void) {
  return (
    <button type="button" data-notice="composer-failure" onClick={onDismiss}
      className={offline
        ? 'mx-1 mt-1.5 flex flex-row items-start gap-2 rounded-xl border border-warning/14 bg-warning/5 px-3 py-2 text-left text-ui-12 text-warning-muted cursor-pointer'
        : 'mx-1 mt-1.5 flex flex-row items-start gap-2 rounded-xl border border-danger/14 bg-danger/5 px-3 py-2 text-left text-ui-12 text-danger-muted cursor-pointer'}>
      <span className="size-3.5 flex-none mt-0.5">{renderIcon('danger-triangle')}</span>
      <span className="min-w-0">{message}</span>
    </button>
  );
}

// The queue notice: one quiet caption line, amber dot only for hard offline.
// Not a warning box, because the amber Notice read as an error.
function renderQueueNotice(text: string, offline: boolean) {
  return (
    <div data-notice="composer-queue-notice" className="relative mx-2 mt-1.5 flex flex-row items-center gap-1.5 text-ui-11 text-text-faint motion-fade-in">
      <span className={offline ? 'size-1.5 flex-none rounded-full bg-warning' : 'size-1.5 flex-none rounded-full bg-text-faint'} />
      <span className="min-w-0 truncate">{text}</span>
    </div>
  );
}

export function Composer({ model, onEvent }: ComposerProps) {
  const [picker, setPicker] = useState<PickerKind | null>(null);
  const [rail, setRail] = useState('claude-code');
  const [active, setActive] = useState(0);
  const [selectedModelId, setSelectedModelId] = useState(model.selectedModelId);
  const [favorites, setFavorites] = useState<readonly string[]>(
    model.models.filter(row => row.favorite).map(row => row.id));
  const [staged, setStaged] = useState(model.attachments);
  const [hoveredAttachment, setHoveredAttachment] = useState<string | null>(null);
  const [preview, setPreview] = useState<string | null>(null);
  const [yolo, setYolo] = useState(model.yolo);
  const [completionActive, setCompletionActive] = useState(0);
  const [failure, setFailure] = useState(model.failure);
  const [picks, setPicks] = useState<readonly number[]>([]);
  const [page, setPage] = useState(0);
  const [hoveredMention, setHoveredMention] = useState<string | null>(null);

  // New chats always use the expanded layout: the target chips need the
  // full-width actions row.
  const isExpanded = model.expanded || model.newChat;
  const previewed = staged.find(attachment => attachment.id === preview);

  function togglePicker(kind: PickerKind) {
    setPicker(picker === kind ? null : kind);
    onEvent(`OpenPicker(${kind})`);
  }
  function toggleFavorite(id: string) {
    setFavorites(favorites.includes(id) ? favorites.filter(row => row !== id) : [...favorites, id]);
    onEvent(`ToggleFavourite(${id})`);
  }

  const input = (
    <ComposerInput lines={model.lines} isPlaceholder={model.isPlaceholder}
      onHoverMention={path => { setHoveredMention(path); onEvent(path === null ? 'MentionTooltipHidden' : `MentionTooltip(${path})`); }} />
  );

  const notices = (
    <>
      {failure !== null && renderFailure(failure, model.failureOffline, () => setFailure(null))}
      {model.queueNotice !== null && renderQueueNotice(model.queueNotice.text, model.queueNotice.offline)}
      {model.steerQueues && (
        <div className="mt-1.5 px-3 text-ui-11 text-text-muted/88">
          This agent cannot be steered mid-turn, so your message is queued and sent when the current turn finishes.
        </div>
      )}
    </>
  );

  // The wizard and a blocked tool each take the composer's place: Render
  // returns on them before it reaches the pill.
  if (model.wizard !== null) {
    const wizard = { page, questions: model.wizard.questions };
    return (
      <section aria-label="Composer" className="w-full max-w-184 mx-auto flex flex-col gap-2 px-4 pb-4 font-sans">
        {notices}
        {renderWizard(wizard, picks, input,
          index => setPicks(picks.includes(index) ? picks.filter(pick => pick !== index) : [...picks, index]),
          () => { setPage(Math.max(0, page - 1)); setPicks([]); },
          () => {
            if (page + 1 >= wizard.questions.length) onEvent('AnswerQuestion');
            else { setPage(page + 1); setPicks([]); }
          })}
      </section>
    );
  }
  if (model.permission !== null) {
    return (
      <section aria-label="Composer" className="w-full max-w-184 mx-auto flex flex-col gap-2 px-4 pb-4 font-sans">
        {notices}
        {renderPermission(model.permission, (id, answer) => onEvent(`AnswerPermission(${id}, ${answer})`))}
      </section>
    );
  }

  const cluster = (
    <>
      <div className="flex-1 min-w-0 flex flex-row items-center justify-end gap-0.5">
        {renderYoloChip(yolo, on => { setYolo(on ? 'on' : 'off'); onEvent(`SetAutoApprove(${on})`); })}
        {renderModelChip(model, picker === 'model', () => togglePicker('model'))}
        {renderAttachButton(() => onEvent('OpenFilePicker'))}
      </div>
      {renderSendButton(model.sendMode, model.sendBlocked,
        () => onEvent('Submit'), () => onEvent('Interrupt'))}
    </>
  );

  const strip = (
    <>
      {renderCommentsChip(model.comments)}
      {renderAttachmentStrip(staged, hoveredAttachment, setHoveredAttachment,
        id => { setPreview(id); onEvent(`PreviewAttachment(${id})`); },
        id => { setStaged(staged.filter(attachment => attachment.id !== id)); onEvent(`RemoveAttachment(${id})`); })}
    </>
  );

  // Expanded: the text box on top, the actions row pinned at the pill's
  // stationary bottom. Compact: input and the cluster on one line.
  const pill = isExpanded ? (
    <div data-pill="expanded" className="w-full flex flex-col rounded-xl border border-border bg-input-bg overflow-hidden motion-fade-quick">
      {strip}
      <div className="min-h-20 px-4 pt-4 pb-1 flex flex-col">{input}</div>
      <div className="h-12 flex flex-row items-center gap-2 pl-3 pr-3 pt-1 pb-2.5">{cluster}</div>
    </div>
  ) : (
    <div data-pill="compact" className="w-full flex flex-col justify-end rounded-full border border-border bg-input-bg overflow-hidden motion-fade-quick">
      {strip}
      <div className="h-12 flex flex-row items-center">
        <div className="flex-1 min-w-0 pl-4 pr-2">{input}</div>
        <div className="flex-none flex flex-row items-center gap-2 pl-1 pr-2">{cluster}</div>
      </div>
    </div>
  );

  return (
    <section aria-label="Composer" className="relative w-full max-w-184 mx-auto flex flex-col gap-2 px-4 pb-4 font-sans">
      {notices}
      {model.newChat && renderTargetSelectors(model, togglePicker)}
      <div className="relative flex flex-col gap-1.5">
        {hoveredMention !== null && <MentionPathTooltip path={hoveredMention} />}
        {model.completion === 'mention' && renderFileMentionPopup(
          model.mentionResults, model.completionLoading, model.completionError,
          completionActive, setCompletionActive, path => onEvent(`AcceptMention(${path})`))}
        {model.completion === 'slash' && renderSlashPopup(
          model.slashCommands, model.completionLoading, model.completionError,
          completionActive, setCompletionActive, name => onEvent(`AcceptSlash(${name})`))}
        {pill}
        {picker === 'model' && (
          <div data-popover="model" className="absolute bottom-12 right-20 w-80 flex flex-col">
            {/* motion-menu-in animates top with fill-mode both, so it must not
                sit on the box that resolves its own position from bottom-12:
                the forced top:0 clipped 116px off the model list. The card
                inside carries the entrance instead. */}
            <div className="relative w-full flex flex-col rounded-xl border border-wash/10 overflow-hidden bg-surface-overlay motion-menu-in">
            {renderHarnessModelPopover(model.harnessTabs, model.models, model.noAgents,
              { rail, active, selectedModelId, favorites },
              next => { setRail(next); setActive(0); },
              setActive,
              id => { setSelectedModelId(id); setPicker(null); onEvent(`PickModel(${id})`); },
              toggleFavorite)}
            </div>
          </div>
        )}
      </div>
      {renderFooter(model, togglePicker)}
      {previewed !== undefined && renderLightbox(previewed, () => setPreview(null))}
    </section>
  );
}
