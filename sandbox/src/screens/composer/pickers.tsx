// Rust: app/crates/ui/src/pickers.rs, Pickers::render (3966), trigger_chip
// (2174), footer_chip (2279), footer_label, render_target_selectors (2371) and
// render_footer (2454); app/crates/ui/src/yolo.rs, chip (132).
import type { ComposerModel, PickerKind, YoloState } from '../../fixtures/composer';
import { renderIcon } from './icons';
import type { IconName } from './icons';

// yolo.rs::YoloState::suffix - the muted second tone on the chip.
export function yoloSuffix(state: YoloState) {
  return state === 'on' ? 'on' : state === 'always' ? 'always' : '';
}

// yolo.rs::chip. The same ghost pill as the model chip. On reads in the
// warning family; Always is not a toggle, so it never brightens.
export function renderYoloChip(state: YoloState, onToggle: (on: boolean) => void) {
  const suffix = yoloSuffix(state);
  const body = (
    <>
      <span className={state === 'on' ? 'size-4 flex-none text-warning' : state === 'always' ? 'size-4 flex-none text-text-faint' : 'size-4 flex-none text-text-muted'}>
        {renderIcon('danger-triangle')}
      </span>
      <span>Yolo</span>
      {suffix !== '' && <span className="text-text-muted/88">{suffix}</span>}
    </>
  );
  if (state === 'always') {
    return (
      <span data-chip="picker-yolo" className="h-8 flex-none flex flex-row items-center gap-1.5 px-2.5 rounded-lg text-ui-12 font-medium text-text-faint">
        {body}
      </span>
    );
  }
  return (
    <button type="button" data-chip="picker-yolo" aria-pressed={state === 'on'}
      onClick={() => onToggle(state !== 'on')}
      className={state === 'on'
        ? 'h-8 flex-none flex flex-row items-center gap-1.5 px-2.5 rounded-lg text-ui-12 font-medium text-warning bg-warning-wash cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover'
        : 'h-8 flex-none flex flex-row items-center gap-1.5 px-2.5 rounded-lg text-ui-12 font-medium text-text-muted cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover'}>
      {body}
    </button>
  );
}

// trigger_chip: the ghost pill the model picker opens from. Brand icon +
// model name, then the joined traits summary as the chip's muted second tone.
export function renderModelChip(
  model: ComposerModel,
  open: boolean,
  onToggle: () => void,
) {
  return (
    <button type="button" data-chip="picker-model" aria-expanded={open} onClick={onToggle}
      className={open
        ? 'h-8 max-w-64 min-w-0 flex flex-row items-center gap-1.5 px-2.5 rounded-lg text-ui-12 font-medium text-text bg-element-hover cursor-pointer'
        : 'h-8 max-w-64 min-w-0 flex flex-row items-center gap-1.5 px-2.5 rounded-lg text-ui-12 font-medium text-text/88 cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover'}>
      <span className={model.noAgents ? 'size-4 flex-none text-text-muted' : 'size-4 flex-none text-claude-brand'}>
        {model.noAgents ? renderIcon('terminal') : renderIcon('claude-mark')}
      </span>
      <span className="min-w-0 truncate">{model.modelLabel}</span>
      {model.modelTraits !== null && (
        <span className={model.traitsCustomized ? 'min-w-0 truncate text-text/88' : 'min-w-0 truncate text-text-muted/88'}>
          {model.modelTraits}
        </span>
      )}
    </button>
  );
}

// footer_chip: a quieter, smaller trigger - leading icon, truncating label,
// trailing chevron.
export function renderFooterChip(
  kind: PickerKind,
  icon: IconName,
  label: string,
  warn: boolean,
  onOpen: (kind: PickerKind) => void,
) {
  return (
    <button type="button" data-chip={`picker-${kind}`} onClick={() => onOpen(kind)}
      className={warn
        ? 'h-5 max-w-64 min-w-0 flex flex-row items-center gap-1.5 px-2 rounded-md text-ui-12 font-medium text-warning cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover'
        : 'h-5 max-w-64 min-w-0 flex flex-row items-center gap-1.5 px-2 rounded-md text-ui-12 font-medium text-text-muted/88 cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover'}>
      <span className="size-3 flex-none text-text-muted/88">{renderIcon(icon)}</span>
      <span className="min-w-0 truncate">{label}</span>
      <span className="size-3 flex-none text-text-muted/50">{renderIcon('alt-arrow-down')}</span>
    </button>
  );
}

// render_target_selectors: device + project, LEFT-aligned, in the row just
// above the pill. New sessions only; existing chats name their target in the
// titlebar.
export function renderTargetSelectors(model: ComposerModel, onOpen: (kind: PickerKind) => void) {
  return (
    <div className="w-full flex flex-row items-center gap-1 px-2.5">
      {renderFooterChip('device', 'monitor', model.device, model.deviceOffline, onOpen)}
      {renderFooterChip('project', 'folder', model.project, false, onOpen)}
    </div>
  );
}

// render_footer: the checkout-kind + ref toolbar under the pill, git spaces
// only. Checkout hugs the left edge, ref the right.
export function renderFooter(model: ComposerModel, onOpen: (kind: PickerKind) => void) {
  if (!model.git) return null;
  return (
    <div className="w-full flex flex-row items-center justify-between gap-2 px-2.5">
      <div className="flex flex-row items-center min-w-0">
        {renderFooterChip('checkout', 'folder', model.checkout, false, onOpen)}
      </div>
      <div className="flex flex-row items-center min-w-0">
        {renderFooterChip('branch', 'git-branch', model.branch, false, onOpen)}
      </div>
    </div>
  );
}
