// Rust: app/crates/ui/src/pickers.rs - render_harness_model_popover (2998) and
// tab_indicator (3654). The combined agent + model switcher: harness tabs
// across the top, the tab's model list under the search, and the pinned traits
// tray at the bottom. The query never leaves the viewed tab.
import type { ReactNode } from 'react';
import { emptyListNote, errorRow, icon, skeletonMenuRows } from './popover';
import { harnessBrandIcon, renderModelRow } from './model_row';
import { renderTraitsSections } from './traits';
import type { TraitsProps } from './traits';
import type { Harness, HarnessId, ModelRow } from '../../fixtures/pickers';

/// The 2px underline marking the viewed tab: it sits on the tab row's bottom
/// hairline. The native indicator hangs 4px below a 32px tab inside the 40px
/// row; here it is a child of the full-height tab cell instead.
function tabIndicator() {
  return <span className="absolute bottom-0 left-1.5 right-1.5 h-0.5 rounded-sm bg-accent" />;
}

export type HarnessModelProps = {
  harnesses: readonly Harness[]; rows: readonly ModelRow[]; query: string;
  favoritesView: boolean; effective: HarnessId | null; locked: boolean;
  active: number; selectedModelId: string | null; favorites: readonly string[];
  catalogLoading: boolean; catalogError: string | null; modelsError: string | null;
  onPickHarness: (harness: HarnessId) => void; onShowFavorites: () => void;
  onActivate: (ix: number) => void; onHover: (ix: number) => void;
  onToggleFavorite: (harness: HarnessId, modelId: string) => void;
  onRetry: () => void;
  traits: TraitsProps;
};

export function renderHarnessModelPopover(props: HarnessModelProps) {
  // Catalog-level loading and error take over the whole card: the tabs ARE the
  // catalog, so there is nothing stable to draw above the skeleton.
  if (props.catalogLoading) return <div className="h-55 p-2">{skeletonMenuRows(5)}</div>;
  if (props.catalogError) {
    return <div className="h-55 p-2">{errorRow(props.catalogError, props.onRetry)}</div>;
  }
  // No-agents empty state: the catalog loaded but offers nothing runnable.
  if (props.harnesses.length === 0) {
    return (
      <div className="p-4 flex flex-col items-center gap-2">
        <span className="text-text-muted">{icon('terminal', 20)}</span>
        <span className="text-ui-13 text-text">No agents available</span>
        <span className="text-ui-12 text-text-muted text-center">
          Enable an installed agent in Settings, then Agents, or install an agent CLI.
        </span>
      </div>
    );
  }
  const searching = props.query.trim().length > 0;
  const listBody: ReactNode = props.rows.length > 0
    ? (
      <div className="size-full overflow-y-scroll px-1.5">
        {props.rows.map((row, ix) => renderModelRow({
          ix, row,
          selected: props.selectedModelId === row.model.id && props.effective === row.harness,
          active: ix === props.active,
          favorite: props.favorites.includes(`${row.harness}:${row.model.id}`),
          compact: !props.favoritesView,
          onActivate: props.onActivate, onHover: props.onHover,
          onToggleFavorite: props.onToggleFavorite,
        }))}
      </div>
    )
    : searching ? emptyListNote('No models found')
      : props.favoritesView ? emptyListNote("No starred models yet - hit a row's star")
        : props.modelsError ? errorRow(props.modelsError, props.onRetry)
          : skeletonMenuRows(5);
  return (
    <div className="flex flex-col">
      <div className="flex-none h-10 px-1.5 border-b border-border flex flex-row items-center gap-0.5">
        <button type="button" data-tab="favorites" onClick={props.onShowFavorites}
          aria-label="Starred models"
          className={props.favoritesView
            ? 'relative w-8 h-10 flex items-center justify-center cursor-pointer text-text'
            : 'relative w-8 h-10 flex items-center justify-center cursor-pointer text-text-faint hover:text-text active:bg-element-active focus:bg-wash/5'}>
          {icon('star-bold', 14)}
          {props.favoritesView && tabIndicator()}
        </button>
        {props.harnesses.map(harness => {
          const viewed = !props.favoritesView && props.effective === harness.id;
          const disabled = props.locked && props.effective !== harness.id;
          const branded = harness.id === 'claude-code';
          return (
            <button key={harness.id} type="button" data-tab={harness.id} disabled={disabled}
              onClick={() => props.onPickHarness(harness.id)} aria-label={harness.name}
              className={disabled
                ? 'relative w-8 h-10 flex items-center justify-center cursor-default opacity-50 text-text-muted'
                : branded
                  ? 'relative w-8 h-10 flex items-center justify-center cursor-pointer text-claude-brand hover:bg-wash/5 active:bg-element-active focus:bg-wash/5'
                  : viewed
                    ? 'relative w-8 h-10 flex items-center justify-center cursor-pointer text-text'
                    : 'relative w-8 h-10 flex items-center justify-center cursor-pointer text-text-muted hover:bg-wash/5 hover:text-text active:bg-element-active focus:bg-wash/5'}>
              {icon(harnessBrandIcon(harness.id), 16)}
              {viewed && tabIndicator()}
            </button>
          );
        })}
      </div>
      <div className="flex-none h-10 px-2.5 border-b border-border flex flex-row items-center gap-2">
        <span className="flex-none text-text-faint">{icon('magnifer', 14)}</span>
        <span className="flex-1 min-w-0 text-ui-13 flex flex-row items-center gap-0.5">
          {props.query
            ? <span className="min-w-0 truncate text-text">{props.query}</span>
            : <span className="min-w-0 truncate text-text-faint">Search models…</span>}
          <span className="flex-none w-0.25 h-3.5 bg-caret" />
        </span>
      </div>
      <div className="relative flex-none h-55 py-1.5 bg-wash/5">{listBody}</div>
      {props.traits.model && (props.traits.model.ladder.length > 0 || props.traits.model.options.length > 0) && (
        <div className="flex-none border-t border-border max-h-55 overflow-y-scroll px-1.5 pb-1.5">
          {renderTraitsSections(props.traits)}
        </div>
      )}
    </div>
  );
}
