// Rust: app/crates/ui/src/settings/shortcuts.rs (ShortcutsPage::render and its
// render_row impl helper, which stays a plain function here). record_key,
// combo_from_keystroke_on, display_combo_on and conflict_owner are ported as
// the same pure functions; combo_from_keystroke_on and display_combo_on come
// from settings.rs. Persisting the keymap and re-applying the app keymap stay
// in Rust: this page emits onChanged(keymap).
import { useState } from 'react';
import type { Keymap, ShortcutDef } from '../../fixtures/settings_devices';
import { fieldLabel, pageColumn, pageHeader, pageSubtitle, sectionCard,
  settingsIcon } from './widgets';

export type ShortcutsProps = {
  catalog: readonly ShortcutDef[];
  keymap: Keymap;
  onChanged: (keymap: Keymap) => void;
};

const SUBTITLE = 'Click a binding, then press the key combination you want to use. '
  + 'Changes apply immediately and stay on this device.';

/// Rust: GROUP_ORDER. The page renders these cards and nothing else.
const GROUP_ORDER = ['Panels', 'Sessions', 'Jump to session'] as const;

/// Rust: group(id). A total match over ShortcutId.
function group(id: string) {
  if (id.startsWith('jumpSession')) return 'Jump to session';
  if (id.startsWith('toggle')) return 'Panels';
  return 'Sessions';
}

/// Rust: description(id). surya lib/shortcuts.ts descriptions, verbatim.
function description(id: string) {
  if (id === 'toggleSidebar') return 'Show or hide sessions and settings navigation.';
  if (id === 'toggleChanges') return 'Show or hide changes for the current session.';
  if (id === 'toggleTerminal') return 'Show or hide the terminal for the current session.';
  if (id === 'toggleFiles') return 'Show or hide the file tree and editor for the current space.';
  if (id === 'toggleTasks') return 'Show or hide the task board for the current space.';
  if (id === 'toggleInbox') return 'Show or hide the needs-you list at the top of the feed.';
  if (id === 'newSession') return 'Open a blank session canvas to start a new session.';
  if (id === 'nextSession') return 'Select the next session in the sidebar, wrapping at the end.';
  if (id === 'prevSession') return 'Select the previous session in the sidebar, wrapping at the start.';
  if (id === 'archiveSession') return 'Move the current session to the archived shelf.';
  return 'Open the session at this place in the sidebar list.';
}

/// Rust: display_combo_on(false, combo) in settings.rs. The sandbox reads as a
/// Windows or Linux client, which is the product platform.
export function displayCombo(combo: string) {
  return combo.split('-').map(part => {
    if (part === 'mod') return 'Ctrl';
    if (part === 'alt') return 'Alt';
    if (part === 'shift') return 'Shift';
    return part.charAt(0).toUpperCase() + part.slice(1);
  }).join('+');
}

const BARE_MODIFIERS = ['ctrl', 'control', 'alt', 'shift', 'cmd', 'meta', 'platform', 'fn', 'os'];

/// Rust: combo_from_keystroke_on(false, ..). Off macOS ctrl IS the primary and
/// stores as "mod". Returns '' for a bare modifier, which stays recording.
export function comboFromKeystroke(ctrl: boolean, alt: boolean, shift: boolean, key: string) {
  const lower = key.trim().toLowerCase();
  if (!lower || BARE_MODIFIERS.includes(lower)) return '';
  const parts = [];
  if (ctrl) parts.push('mod');
  if (alt) parts.push('alt');
  if (shift) parts.push('shift');
  parts.push(lower);
  return parts.join('-');
}

/// Rust: conflict_owner(keymap, id, combo). The shortcut already bound to this
/// combo, if any; conflicts are refused at record time and never persist.
export function conflictOwner(catalog: readonly ShortcutDef[], keymap: Keymap,
  id: string, combo: string) {
  return catalog.find(entry => entry.id !== id && keymap[entry.id] === combo);
}

/// Rust: ShortcutsPage::render_row. Label and description left, Reset when the
/// row is customized, and the click-to-record combo chip.
function renderRow(entry: ShortcutDef, first: boolean, combo: string, recording: boolean,
  onRecord: () => void, onKey: (event: { key: string; ctrlKey: boolean; altKey: boolean;
    shiftKey: boolean; preventDefault: () => void }) => void, onReset: () => void) {
  const nonDefault = combo !== entry.defaultCombo;
  return (
    <div key={entry.id} data-shortcut={entry.id} className={first
      ? 'min-h-16 px-5 flex flex-row items-center gap-5'
      : 'min-h-16 px-5 flex flex-row items-center gap-5 border-t border-border'}>
      <div className="flex-1 min-w-0 flex flex-col">
        <div className="text-ui-13 font-medium text-text">{entry.label}</div>
        <div className="mt-0.5 text-ui-12 text-text-muted">{description(entry.id)}</div>
      </div>
      {nonDefault && !recording && (
        <button type="button" onClick={onReset} aria-label={`Reset ${entry.label}`}
          className="flex-none text-ui-11 text-text-faint cursor-pointer hover:text-text active:text-text focus:text-text">
          Reset
        </button>
      )}
      <button type="button" onClick={onRecord} onKeyDown={onKey}
        aria-label={`Record a shortcut for ${entry.label}`} data-combo={entry.id}
        className={recording
          ? 'flex-none min-w-24 px-3 py-1.5 rounded-lg border border-text/50 flex justify-center text-ui-12 cursor-pointer bg-text text-on-solid'
          : 'flex-none min-w-24 px-3 py-1.5 rounded-lg border border-border flex justify-center text-ui-12 cursor-pointer bg-bg text-text hover:border-text/50 hover:bg-wash/5 active:bg-wash/10 focus:border-text/50 focus:bg-wash/5'}>
        {recording ? 'Press keys…' : displayCombo(combo)}
      </button>
    </div>
  );
}

export function ShortcutsPage({ catalog, keymap, onChanged }: ShortcutsProps) {
  const [working, setWorking] = useState<Keymap>(keymap);
  const [recording, setRecording] = useState('');
  const [notice, setNotice] = useState('');
  const customized = catalog.some(entry => working[entry.id] !== entry.defaultCombo);

  function commit(next: Keymap) {
    setWorking(next);
    onChanged(next);
  }

  function onKey(entry: ShortcutDef, event: { key: string; ctrlKey: boolean; altKey: boolean;
    shiftKey: boolean; preventDefault: () => void }) {
    if (recording !== entry.id) return;
    event.preventDefault();
    if (event.key.toLowerCase() === 'escape') { setRecording(''); return; }
    const combo = comboFromKeystroke(event.ctrlKey, event.altKey, event.shiftKey, event.key);
    if (!combo) return;
    const owner = conflictOwner(catalog, working, entry.id, combo);
    if (owner) {
      setNotice(`${displayCombo(combo)} is already assigned to ${owner.label}.`);
      setRecording('');
      return;
    }
    setRecording('');
    setNotice('');
    commit({ ...working, [entry.id]: combo });
  }

  function reset(entry: ShortcutDef) {
    setRecording('');
    commit({ ...working, [entry.id]: entry.defaultCombo });
  }

  function restoreDefaults() {
    setRecording('');
    setNotice('');
    commit(Object.fromEntries(catalog.map(entry => [entry.id, entry.defaultCombo])));
  }

  let helper = 'Shortcuts must be unique.';
  if (recording) helper = 'Press Escape to cancel.';
  else if (notice) helper = notice;
  const disabled = !customized || recording !== '';

  return (
    <div className="size-full overflow-y-scroll">
      {pageColumn(<>
        <div className="flex flex-row items-start justify-between gap-6">
          <div className="flex flex-col">
            {pageHeader('Keyboard shortcuts')}
            {pageSubtitle(SUBTITLE, true)}
          </div>
          <button type="button" onClick={disabled ? undefined : restoreDefaults}
            disabled={disabled} data-action="Restore defaults"
            className={disabled
              ? 'flex-none flex flex-row items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-ui-12 text-text-muted opacity-50'
              : 'flex-none flex flex-row items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-ui-12 text-text-muted cursor-pointer hover:bg-wash/5 hover:text-text active:bg-wash/10 focus:bg-wash/5 focus:text-text'}>
            <span className="flex-none size-3.5 flex">{settingsIcon('refresh')}</span>
            Restore defaults
          </button>
        </div>
        <div className="mt-8 flex flex-col gap-7">
          {GROUP_ORDER.map(name => (
            <div key={name} className="flex flex-col gap-3">
              {fieldLabel(name)}
              {sectionCard(catalog.filter(entry => group(entry.id) === name).map((entry, index) =>
                renderRow(entry, index === 0, working[entry.id], recording === entry.id,
                  () => { setRecording(entry.id); setNotice(''); },
                  event => onKey(entry, event), () => reset(entry))))}
            </div>
          ))}
        </div>
        <div className="mt-3 px-1 min-h-5 text-ui-12 text-text-muted">{helper}</div>
      </>)}
    </div>
  );
}
