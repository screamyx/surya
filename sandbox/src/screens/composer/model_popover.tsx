// Rust: app/crates/ui/src/pickers.rs, Pickers::render_harness_model_popover
// (2998) and Pickers::render_model_row (3325).
import type { HarnessTab, ModelRow } from '../../fixtures/composer';
import { renderIcon } from './icons';
import { kbdHint, emptyNote } from './chrome';
import { jumpLabel } from './key_chips';

export type ModelPopoverState = {
  rail: string;
  active: number;
  selectedModelId: string;
  favorites: readonly string[];
};

// The favorites star, then one brand icon per harness, across the top. The
// viewed tab wears a 2px accent bar on the row's bottom hairline.
function renderTabs(
  tabs: readonly HarnessTab[],
  rail: string,
  onRail: (rail: string) => void,
) {
  return (
    <div className="flex-none h-10 px-1.5 border-b border-wash/10 flex flex-row items-center gap-0.5">
      <button type="button" data-tab="favorites" aria-label="Starred models" onClick={() => onRail('favorites')}
        className={rail === 'favorites'
          ? 'relative size-8 flex-none flex items-center justify-center rounded-lg cursor-pointer'
          : 'relative size-8 flex-none flex items-center justify-center rounded-lg cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover'}>
        <span className={rail === 'favorites' ? 'size-3.75 text-text' : 'size-3.75 text-text-muted/88'}>
          {renderIcon('star-bold')}
        </span>
        {rail === 'favorites' && <span className="absolute bottom-0 left-1.5 right-1.5 h-0.5 rounded-sm bg-accent" />}
      </button>
      {tabs.map(tab => (
        <button key={tab.id} type="button" data-tab={tab.id} aria-label={tab.name}
          onClick={() => onRail(tab.id)} aria-disabled={tab.disabled ? 'true' : undefined}
          className={tab.disabled
            ? 'relative size-8 flex-none flex items-center justify-center rounded-lg opacity-50'
            : 'relative size-8 flex-none flex items-center justify-center rounded-lg cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover'}>
          <span className={tab.id === 'claude-code'
            ? 'size-4 text-claude-brand'
            : 'size-4 text-text-muted'}>
            {tab.id === 'claude-code' ? renderIcon('claude-mark') : renderIcon('terminal')}
          </span>
          {rail === tab.id && <span className="absolute bottom-0 left-1.5 right-1.5 h-0.5 rounded-sm bg-accent" />}
        </button>
      ))}
    </div>
  );
}

// render_model_row. Compact single-line rows on a harness tab; the favorites
// tab mixes harnesses and keeps the two-line layout with the brand subline.
function renderModelRow(
  row: ModelRow,
  index: number,
  compact: boolean,
  selected: boolean,
  active: boolean,
  favorite: boolean,
  onPick: () => void,
  onHover: () => void,
  onStar: () => void,
) {
  const body = compact ? (
    <span className="flex-1 min-w-0 flex flex-row items-center gap-1.5">
      <span className="flex-none max-w-full truncate text-ui-12 font-medium text-text">{row.label}</span>
      {row.attribution !== '' && (
        <span className="min-w-0 truncate text-ui-11 text-text-muted/88">{row.attribution}</span>
      )}
    </span>
  ) : (
    <span className="flex-1 min-w-0 flex flex-col gap-0.5">
      <span className="w-full truncate text-ui-12 font-medium text-text">{row.label}</span>
      <span className="flex flex-row items-center gap-1.5">
        <span className={row.harnessId === 'claude-code' ? 'size-3 flex-none text-claude-brand' : 'size-3 flex-none text-text-muted/88'}>
          {row.harnessId === 'claude-code' ? renderIcon('claude-mark') : renderIcon('terminal')}
        </span>
        <span className="flex-none text-ui-11 text-text-muted/88">{row.harnessName}</span>
        {row.attribution !== '' && <span className="flex-none text-ui-11 text-text-muted/50">·</span>}
        {row.attribution !== '' && <span className="min-w-0 truncate text-ui-11 text-text-muted/88">{row.attribution}</span>}
      </span>
    </span>
  );
  return (
    <div key={`${row.harnessId}:${row.id}`} className="pb-0.5 flex">
      <div data-model={row.id} onMouseEnter={onHover}
        className={selected
          ? 'w-full flex flex-row items-center gap-2.5 px-2 py-1 rounded-md cursor-pointer bg-wash/10'
          : active
            ? 'w-full flex flex-row items-center gap-2.5 px-2 py-1 rounded-md cursor-pointer bg-wash/5'
            : 'w-full flex flex-row items-center gap-2.5 px-2 py-1 rounded-md cursor-pointer'}>
        <button type="button" aria-label={`Use ${row.label}`} onClick={onPick}
          className="flex-1 min-w-0 flex flex-row items-center text-left cursor-pointer">
          {body}
        </button>
        {index < 9 && kbdHint(jumpLabel(false, index + 1))}
        <button type="button" data-star={row.id} aria-label={favorite ? `Unstar ${row.label}` : `Star ${row.label}`}
          onClick={onStar}
          className="flex-none size-5 flex items-center justify-center rounded-md cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover">
          <span className={favorite ? 'size-3.5 text-warning' : 'size-3.5 text-text-muted/50'}>
            {favorite ? renderIcon('star-bold') : renderIcon('star')}
          </span>
        </button>
      </div>
    </div>
  );
}

export function renderHarnessModelPopover(
  tabs: readonly HarnessTab[],
  models: readonly ModelRow[],
  noAgents: boolean,
  state: ModelPopoverState,
  onRail: (rail: string) => void,
  onActive: (index: number) => void,
  onPick: (id: string) => void,
  onStar: (id: string) => void,
) {
  // The catalog loaded but offers nothing runnable: guidance, not an empty
  // tab row.
  if (noAgents) {
    return (
      <div className="p-4 flex flex-col items-center gap-2">
        <span className="size-5 text-text-muted">{renderIcon('terminal')}</span>
        <span className="text-ui-13 text-text">No agents available</span>
        <span className="text-ui-12 text-text-muted text-center">
          Enable an installed agent in Settings, Agents, or install an agent CLI.
        </span>
      </div>
    );
  }
  const favoritesView = state.rail === 'favorites';
  const rows = favoritesView
    ? models.filter(model => state.favorites.includes(model.id))
    : models.filter(model => model.harnessId === state.rail);
  return (
    <div className="flex flex-col">
      {renderTabs(tabs, state.rail, onRail)}
      <div className="flex-none h-10 px-2.5 border-b border-wash/10 flex flex-row items-center gap-2">
        <span className="size-3.5 flex-none text-text-muted/88">{renderIcon('magnifer')}</span>
        <span className="flex-1 min-w-0 text-ui-13 text-text-faint">Search this agent</span>
      </div>
      <div className="flex-none h-55 py-1.5 px-1.5 flex flex-col overflow-y-scroll bg-wash/5">
        {rows.length === 0
          ? emptyNote(favoritesView ? 'No starred models yet, hit a row star' : 'No models found')
          : rows.map((row, index) => renderModelRow(
            row, index, !favoritesView,
            row.id === state.selectedModelId,
            index === state.active,
            state.favorites.includes(row.id),
            () => onPick(row.id), () => onActive(index), () => onStar(row.id),
          ))}
      </div>
    </div>
  );
}
