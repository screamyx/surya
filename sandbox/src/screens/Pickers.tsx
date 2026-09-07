// Rust: app/crates/ui/src/pickers.rs (Pickers). `impl Render for Pickers` at
// line 3966 is the composer's ACTIONS row alone; render_target_selectors (2371)
// and render_footer (2454) are the entity's two other public render entry
// points, called by composer.rs. All three are stacked here in the order the
// composer paints them, so the five popovers can be opened side by side.
import { useState } from 'react';
import { footerChip, overlayAbove, triggerChip, yoloChip } from './pickers/chips';
import { popoverCard, popoverCardFlush } from './pickers/popover';
import { renderBranchPopover } from './pickers/branch';
import { renderCheckoutPopover } from './pickers/checkout';
import { renderDevicePopover, renderSpacePopover } from './pickers/space_device';
import { renderHarnessModelPopover } from './pickers/harness_model';
import { renderTargetSelectors } from './pickers/target_selectors';
import { renderFooter } from './pickers/footer';
import { defaultReasoning } from './pickers/traits';
import { harnessBrandIcon } from './pickers/model_row';
import {
  MAX_REF_ROWS, NO_ACTIVE_ROW, favoriteKey, filterBy, menuStep, scopedModelRows,
  traitsCustomized, traitsSummary,
} from './pickers/rows';
import type {
  CheckoutKind, HarnessId, PickersFixture, ReasoningLevel,
} from '../fixtures/pickers';

/// PickerKind: which picker popover is open.
export type PickerKind = 'branch' | 'checkout' | 'harness-model' | 'space' | 'device';

export type PickersProps = {
  fixture: PickersFixture;
  /// A committed chat locks the harness: the other tabs stay visible but
  /// disabled, so the lock reads as a rule.
  locked?: boolean;
  /// Everything that reaches the engine leaves as a named intention.
  onPickModel: (harness: HarnessId, modelId: string) => void;
  onPickRef: (name: string) => void;
  onPickSpace: (spaceId: string) => void;
  onPickDevice: (deviceId: string) => void;
  onNewProject: () => void;
  onSetAutoApprove: (on: boolean) => void;
  onRetry: (kind: PickerKind) => void;
};

export function Pickers(props: PickersProps) {
  const fixture = props.fixture;
  const [open, setOpen] = useState<PickerKind | null>(null);
  const [active, setActive] = useState(NO_ACTIVE_ROW);
  const [query, setQuery] = useState(fixture.query);
  const [favoritesView, setFavoritesView] = useState(false);
  const [favorites, setFavorites] = useState<readonly string[]>(fixture.favorites);
  const [harness, setHarness] = useState<HarnessId | null>(fixture.selectedHarness);
  const [modelId, setModelId] = useState<string | null>(fixture.selectedModelId);
  const [reasoning, setReasoning] = useState<ReasoningLevel | null>(null);
  const [options, setOptions] = useState<Readonly<Record<string, string>>>({});
  const [spaceId, setSpaceId] = useState<string | null>(fixture.selectedSpaceId);
  const [deviceId, setDeviceId] = useState<string | null>(fixture.selectedDeviceId);
  const [branch, setBranch] = useState<string | null>(fixture.branch);
  const [checkout, setCheckout] = useState<CheckoutKind>(fixture.checkout);
  const [yolo, setYolo] = useState(fixture.yolo);

  const modelsFor = (id: HarnessId) => fixture.models[id] ?? [];
  const isFavorite = (id: HarnessId, model: string) => favorites.includes(favoriteKey(id, model));
  const modelRows = scopedModelRows(query, favoritesView, harness, fixture.harnesses, modelsFor, isFavorite);
  const selectedModel = harness ? modelsFor(harness).find(model => model.id === modelId) ?? null : null;
  const ladderDefault = selectedModel ? defaultReasoning(selectedModel.ladder) : null;
  const effectiveReasoning = reasoning ?? ladderDefault;

  const refRows = filterBy(query, fixture.refs, row => row.name).slice(0, MAX_REF_ROWS);
  const spaceRows = filterBy(query, fixture.spaces, row => row.name);
  const deviceRows = filterBy(query, fixture.devices, row => row.name);
  const space = fixture.spaces.find(row => row.id === spaceId) ?? null;
  const device = fixture.devices.find(row => row.id === deviceId) ?? null;
  const worktreeRef = fixture.refs.find(row => row.name === branch);
  const hasWorktree = worktreeRef ? worktreeRef.worktree : false;

  const rowCount = open === 'branch' ? refRows.length
    : open === 'checkout' ? 2
      : open === 'harness-model' ? modelRows.length
        : open === 'space' ? spaceRows.length
          : open === 'device' ? deviceRows.length : 0;

  // toggle (926): opening re-primes the highlight and clears the shared query.
  function toggle(kind: PickerKind) {
    setQuery('');
    setActive(NO_ACTIVE_ROW);
    setFavoritesView(false);
    setOpen(open === kind ? null : kind);
  }

  function activateRow(index: number) {
    if (open === 'harness-model') {
      const row = modelRows[index];
      if (row) { setHarness(row.harness); setModelId(row.model.id); setReasoning(null); setOptions({}); props.onPickModel(row.harness, row.model.id); }
    } else if (open === 'checkout') {
      pickCheckout(index === 0 ? 'local' : 'new-worktree');
    } else if (open === 'branch') {
      const row = refRows[index];
      if (row) pickRef(row.name);
    } else if (open === 'space') {
      const row = spaceRows[index];
      if (row) pickSpace(row.id);
    } else if (open === 'device') {
      const row = deviceRows[index];
      if (row) pickDevice(row.id);
    }
  }

  function pickRef(name: string) { setBranch(name); setOpen(null); props.onPickRef(name); }
  function pickCheckout(kind: CheckoutKind) { setCheckout(kind); setOpen(null); }
  function pickSpace(id: string) { setSpaceId(id); setOpen(null); props.onPickSpace(id); }
  function pickDevice(id: string) { setDeviceId(id); setOpen(null); props.onPickDevice(id); }
  function pickHarness(id: HarnessId) {
    setHarness(id); setFavoritesView(false); setActive(NO_ACTIVE_ROW);
    const first = modelsFor(id)[0];
    if (first) { setModelId(first.id); setReasoning(null); setOptions({}); }
  }
  function toggleFavorite(id: HarnessId, model: string) {
    const key = favoriteKey(id, model);
    setFavorites(favorites.includes(key) ? favorites.filter(entry => entry !== key) : favorites.concat(key));
  }

  // on_key_down (2098). The query input is GPUI-painted with no browser pair,
  // so typing is read off the popover container instead of an <input>.
  function onKeyDown(event: { key: string; ctrlKey: boolean; metaKey: boolean; preventDefault: () => void }) {
    if (!open) return;
    if (open === 'harness-model' && (event.ctrlKey || event.metaKey) && /^[1-9]$/.test(event.key)) {
      event.preventDefault();
      activateRow(Number(event.key) - 1);
      return;
    }
    if (event.ctrlKey || event.metaKey) return;
    if (event.key === 'Escape') { event.preventDefault(); setOpen(null); return; }
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      setActive(menuStep(active, rowCount, event.key === 'ArrowDown' ? 1 : -1));
      return;
    }
    if (event.key === 'Enter') {
      event.preventDefault();
      if (active !== NO_ACTIVE_ROW) activateRow(active);
      return;
    }
    if (event.key === 'Backspace') {
      event.preventDefault();
      setQuery(query.slice(0, -1));
      setActive(NO_ACTIVE_ROW);
      return;
    }
    if (event.key.length === 1) {
      event.preventDefault();
      setQuery(query + event.key);
      setActive(NO_ACTIVE_ROW);
    }
  }

  const catalogLoading = false;
  const noAgents = fixture.harnesses.length === 0;
  const modelLabel = noAgents ? 'No agents available' : selectedModel ? selectedModel.label : '';
  const traits = traitsSummary(selectedModel, effectiveReasoning, options);
  const traitsActive = traitsCustomized(selectedModel, effectiveReasoning, ladderDefault, options);

  const harnessModelPopover = popoverCardFlush(renderHarnessModelPopover({
    harnesses: fixture.harnesses, rows: modelRows, query, favoritesView,
    effective: harness, locked: props.locked === true, active,
    selectedModelId: modelId, favorites, catalogLoading, catalogError: null,
    modelsError: null,
    onPickHarness: pickHarness, onShowFavorites: () => { setFavoritesView(true); setActive(NO_ACTIVE_ROW); },
    onActivate: activateRow, onHover: setActive, onToggleFavorite: toggleFavorite,
    onRetry: () => props.onRetry('harness-model'),
    traits: {
      model: selectedModel, reasoning: effectiveReasoning, options,
      onPickReasoning: setReasoning,
      onPickOption: (optionId, choiceId) => setOptions({ ...options, [optionId]: choiceId }),
    },
  }));

  return (
    <section aria-label="Composer pickers" onKeyDown={onKeyDown}
      className="w-full flex flex-col gap-2 py-4 bg-surface text-text font-sans leading-gpui">
      {renderTargetSelectors({
        deviceLabel: device ? device.name : 'This device',
        projectLabel: space ? space.name : 'No project',
        deviceOffline: device ? !device.online : false,
        openKind: open,
        onToggleDevice: () => toggle('device'),
        onToggleSpace: () => toggle('space'),
        devicePopover: popoverCard(
          <div className="w-55">{renderDevicePopover({
            rows: deviceRows, query, selected: deviceId, active,
            onPick: pickDevice, onHover: setActive,
          })}</div>),
        spacePopover: popoverCard(
          <div className="w-64">{renderSpacePopover({
            rows: spaceRows, query, selected: spaceId, active,
            onPick: pickSpace, onHover: setActive,
            onNewProject: () => { setOpen(null); props.onNewProject(); },
          })}</div>),
      })}
      {renderFooter({
        git: space ? space.gitDetected : false,
        locked: props.locked === true,
        checkout, hasWorktree, branch, openKind: open,
        onToggleCheckout: () => toggle('checkout'),
        onToggleBranch: () => toggle('branch'),
        checkoutPopover: popoverCard(
          <div className="w-55">{renderCheckoutPopover({
            current: checkout, hasWorktree, active,
            onPick: pickCheckout, onHover: setActive,
          })}</div>),
        branchPopover: popoverCard(
          <div className="w-80">{renderBranchPopover({
            rows: refRows, total: fixture.refTotal, hasSpace: space !== null, query,
            selected: branch, active, switching: null, switchError: fixture.switchError,
            loading: false, error: null,
            onPick: pickRef, onHover: setActive, onRetry: () => props.onRetry('branch'),
          })}</div>),
      })}
      <div className="w-full min-w-0 flex flex-row items-center justify-between gap-2">
        <div className="flex flex-row items-center min-w-0 gap-1" />
        <div className="flex flex-row items-center min-w-0 gap-1">
          {yoloChip(yolo, () => { const next = yolo === 'on' ? 'off' : 'on'; setYolo(next); props.onSetAutoApprove(next === 'on'); })}
          <div className="relative flex flex-row items-center min-w-0">
            {triggerChip('picker-model', modelLabel, true,
              noAgents ? 'terminal' : harness ? harnessBrandIcon(harness) : 'claude-mark',
              harness === 'claude-code' || (!noAgents && harness === null),
              false, !noAgents && modelLabel === '', traits, traitsActive,
              open === 'harness-model', () => toggle('harness-model'))}
            {open === 'harness-model' && overlayAbove('end', <div className="w-80">{harnessModelPopover}</div>)}
          </div>
        </div>
      </div>
    </section>
  );
}
