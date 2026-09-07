// Rust: app/crates/proto/src/match_rank.rs (match_rank),
// app/crates/ui/src/popover.rs - filter_indices (239) and menu_step (214), and
// app/crates/ui/src/pickers.rs - scoped_model_rows (3665), filtered_ref_rows
// (1679), filtered_space_rows (1798), filtered_device_rows (1880).
// Pure ranking helpers, no JSX.
import type { Harness, HarnessId, ModelRow, PickerModel } from '../../fixtures/pickers';

/// match_rank: 0 prefix, 1 substring, null no match. Case-insensitive; an empty
/// query matches everything at rank 1 and keeps input order.
export function matchRank(query: string, label: string): number | null {
  const q = query.trim().toLowerCase();
  if (q.length === 0) return 1;
  const l = label.toLowerCase();
  if (l.startsWith(q)) return 0;
  return l.includes(q) ? 1 : null;
}

/// filter_indices: prefix matches first, then substring, stable within a rank.
export function filterBy<T>(query: string, items: readonly T[], label: (item: T) => string): T[] {
  const ranked: { rank: number; ix: number; item: T }[] = [];
  items.forEach((item, ix) => {
    const rank = matchRank(query, label(item));
    if (rank !== null) ranked.push({ rank, ix, item });
  });
  ranked.sort((a, b) => (a.rank - b.rank) || (a.ix - b.ix));
  return ranked.map(entry => entry.item);
}

/// menu_step: Down from nothing lands on row 0, Up from nothing on the last
/// row, and both wrap.
export function menuStep(active: number, count: number, delta: number): number {
  if (count === 0) return NO_ACTIVE_ROW;
  if (active === NO_ACTIVE_ROW) return delta >= 0 ? 0 : count - 1;
  return ((active + delta) % count + count) % count;
}

/// Sentinel for "no keyboard-highlighted row" (Rust NO_ACTIVE_ROW).
export const NO_ACTIVE_ROW = -1;

/// The ref list is capped before it paints (Rust MAX_REF_ROWS).
export const MAX_REF_ROWS = 300;

export function favoriteKey(harness: HarnessId, modelId: string) {
  return `${harness}:${modelId}`;
}

/// scoped_model_rows: the QUERY NEVER LEAVES THE VIEWED TAB. On a harness tab
/// it ranks that harness's models only, on the favorites tab the starred set.
/// Without a query a harness tab lists its catalog stars-first.
export function scopedModelRows(
  query: string, favoritesView: boolean, effective: HarnessId | null,
  harnesses: readonly Harness[], modelsFor: (harness: HarnessId) => readonly PickerModel[],
  isFavorite: (harness: HarnessId, modelId: string) => boolean,
): ModelRow[] {
  const row = (harness: Harness, model: PickerModel): ModelRow =>
    ({ harness: harness.id, harnessName: harness.name, model });
  const inScope = (harness: Harness, model: PickerModel) => favoritesView
    ? isFavorite(harness.id, model.id)
    : harness.id === effective;
  if (query.trim().length > 0) {
    // Rank: label prefix, label substring, then description hit; stars, then
    // input order, break ties. The description stays in the haystack because a
    // provider attribution must find its models even inside one tab.
    const ranked: { rank: number; unstarred: number; ix: number; row: ModelRow }[] = [];
    let inputIx = 0;
    harnesses.forEach(harness => {
      modelsFor(harness.id).forEach(model => {
        if (inScope(harness, model)) {
          const byLabel = matchRank(query, model.label);
          const described = matchRank(query, `${model.description ?? ''} ${model.label}`);
          const ranks: number[] = [];
          if (byLabel !== null) ranks.push(byLabel);
          if (described !== null) ranks.push(described + 2);
          if (ranks.length > 0) {
            ranked.push({
              rank: Math.min(...ranks),
              unstarred: isFavorite(harness.id, model.id) ? 0 : 1,
              ix: inputIx, row: row(harness, model),
            });
          }
        }
        inputIx += 1;
      });
    });
    ranked.sort((a, b) => (a.rank - b.rank) || (a.unstarred - b.unstarred) || (a.ix - b.ix));
    return ranked.map(entry => entry.row);
  }
  if (favoritesView) {
    const rows: ModelRow[] = [];
    harnesses.forEach(harness => {
      modelsFor(harness.id).forEach(model => {
        if (isFavorite(harness.id, model.id)) rows.push(row(harness, model));
      });
    });
    return rows;
  }
  const viewed = harnesses.find(harness => harness.id === effective);
  if (!viewed) return [];
  const models = modelsFor(viewed.id);
  const starred = models.filter(model => isFavorite(viewed.id, model.id));
  const rest = models.filter(model => !isFavorite(viewed.id, model.id));
  return starred.concat(rest).map(model => row(viewed, model));
}

/// traits_summary: the effective reasoning level plus every option's effective
/// choice, joined with the middle dot. Null when the model describes nothing.
export function traitsSummary(
  model: PickerModel | null, reasoning: string | null, options: Readonly<Record<string, string>>,
): string | null {
  if (!model) return null;
  const parts: string[] = [];
  if (reasoning) parts.push(reasoning);
  model.options.forEach(option => {
    const pickedId = options[option.id] ?? option.defaultChoice;
    const choice = option.choices.find(entry => entry.id === pickedId);
    if (choice) parts.push(choice.label);
  });
  return parts.length > 0 ? parts.join(' · ') : null;
}

/// traits_customized: whether anything departs from its default, which is what
/// brightens the chip's suffix.
export function traitsCustomized(
  model: PickerModel | null, reasoning: string | null,
  fallback: string | null, options: Readonly<Record<string, string>>,
): boolean {
  if (!model) return false;
  if (reasoning !== null && fallback !== null && reasoning !== fallback) return true;
  return model.options.some(option => (options[option.id] ?? option.defaultChoice) !== option.defaultChoice);
}
