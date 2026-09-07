// Rust: app/crates/ui/src/pickers.rs - render_model_row (3325). One row of the
// virtualized model list. `ix` is the row's GLOBAL index: the Ctrl+N chips, the
// hover cursor and activation all key on it. The 2px inter-row gap is baked
// into each item's bottom padding so every item is the same height.
import { icon, kbdHint } from './popover';
import type { IconName } from './icons';
import type { HarnessId, ModelRow } from '../../fixtures/pickers';

/// harness_brand_icon (3856): the Claude mark keeps its brand orange even on
/// the monochrome surface; every other mark is tinted by the surface.
export function harnessBrandIcon(harness: HarnessId): IconName {
  return harness === 'claude-code' ? 'claude-mark'
    : harness === 'opencode' ? 'opencode-mark' : 'cursor-mark';
}

/// key_chips::jump_label(mac, slot). Windows is the product, so the chip reads
/// Ctrl+N; the Mac Command glyph never reaches a Windows chip (E2E-UI-04).
export function jumpLabel(slot: number) {
  return `Ctrl+${slot}`;
}

export type ModelRowProps = {
  ix: number; row: ModelRow; selected: boolean; active: boolean; favorite: boolean;
  /// A harness tab's rows all share one harness, so the identity subline is
  /// dead weight there and the row collapses to a single compact line.
  compact: boolean;
  onActivate: (ix: number) => void; onHover: (ix: number) => void;
  onToggleFavorite: (harness: HarnessId, modelId: string) => void;
};

export function renderModelRow(props: ModelRowProps) {
  const row = props.row;
  const mark = harnessBrandIcon(row.harness);
  const branded = row.harness === 'claude-code';
  // Provider attribution: several connected opencode providers advertise
  // identically-named models, so the row names its provider. Skipped when it
  // just repeats the harness name.
  const described = row.model.description ? row.model.description.trim() : '';
  const attribution = described && described.toLowerCase() !== row.harnessName.toLowerCase()
    ? described : null;
  const body = props.compact
    ? (
      <div className="flex-1 min-w-0 flex flex-row items-center gap-1.5">
        <span className="flex-none max-w-full truncate text-ui-13 font-medium text-text">{row.model.label}</span>
        {attribution && <span className="min-w-0 truncate text-ui-11 text-text-faint">{attribution}</span>}
      </div>
    )
    : (
      <div className="flex-1 min-w-0 flex flex-col gap-0.5">
        <span className="w-full truncate text-ui-13 font-medium text-text">{row.model.label}</span>
        <span className="flex flex-row items-center gap-1.5">
          <span className={branded ? 'flex-none text-claude-brand' : 'flex-none text-text-faint'}>{icon(mark, 11)}</span>
          <span className="flex-none text-ui-11 text-text-faint">{row.harnessName}</span>
          {attribution && <span className="flex-none text-ui-11 text-text-faint">·</span>}
          {attribution && <span className="min-w-0 truncate text-ui-11 text-text-faint">{attribution}</span>}
        </span>
      </div>
    );
  // ONE moving highlight: hovering moves the keyboard cursor instead of
  // painting its own wash, so hover and the arrow cursor can never wear two
  // washes at once. Selection is the distinct stronger treatment.
  return (
    <div key={`model-row-${props.ix}`} className="pb-0.5">
      <div data-row={`model-row-${props.ix}`} onMouseEnter={() => props.onHover(props.ix)}
        className={props.selected
          ? 'px-2 py-1.5 rounded-md flex flex-row items-center gap-2.5 cursor-pointer bg-wash/10'
          : props.active
            ? 'px-2 py-1.5 rounded-md flex flex-row items-center gap-2.5 cursor-pointer bg-wash/5'
            : 'px-2 py-1.5 rounded-md flex flex-row items-center gap-2.5 cursor-pointer'}>
        <button type="button" onClick={() => props.onActivate(props.ix)}
          aria-label={`Pick ${row.model.label}`}
          className="flex-1 min-w-0 flex flex-row items-center text-left cursor-pointer">
          {body}
        </button>
        {props.ix < 9 && kbdHint(jumpLabel(props.ix + 1))}
        <button type="button" aria-label={`Star ${row.model.label}`}
          onClick={() => props.onToggleFavorite(row.harness, row.model.id)}
          className={props.favorite
            ? 'flex-none size-5 rounded-md flex items-center justify-center cursor-pointer text-warning hover:bg-wash/10 active:bg-element-active focus:bg-wash/10'
            : 'flex-none size-5 rounded-md flex items-center justify-center cursor-pointer text-text-faint hover:bg-wash/10 active:bg-element-active focus:bg-wash/10'}>
          {props.favorite ? icon('star-bold', 14) : icon('star', 14)}
        </button>
      </div>
    </div>
  );
}
