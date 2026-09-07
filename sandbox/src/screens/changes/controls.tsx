// Rust: app/crates/ui/src/changes.rs, Changes::render_header_controls,
// Changes::header_button, Changes::header_toggle, Changes::split_toggle,
// Changes::wrap_toggle and Changes::render_scope_menu.
// Native mounts these in the session titlebar's trailing section, not inside
// the pane: the titlebar overlay owns that strip's hit-testing. The sandbox
// has no titlebar seam for a pane to reach into, so the pane renders them at
// the top of its own frame.
import type { DiffMode, DiffScope } from './model';
import { SCOPE_MENU, scopeMenuLabel } from './model';
import { DiffHeaderTooltip } from './tooltip';

/// Rust: `crate::icons::SPLIT_COLUMNS`, `WRAP_TEXT`, `FOLD_VERTICAL`, from
/// app/crates/ui/assets/icons. Inlined for the same reason as the chevrons.
function toolbarIcon(name: 'split' | 'wrap' | 'fold') {
  if (name === 'split') {
    return (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><g fill="none" stroke="currentColor" strokeWidth="1.5"><path d="M2 11c0-3.771 0-5.657 1.172-6.828S6.229 3 10 3h4c3.771 0 5.657 0 6.828 1.172S22 7.229 22 11v2c0 3.771 0 5.657-1.172 6.828S17.771 21 14 21h-4c-3.771 0-5.657 0-6.828-1.172S2 16.771 2 13z" /><path strokeLinecap="round" d="M12 21V3" /></g></svg>);
  }
  if (name === 'wrap') {
    return (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><g fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M4 6H20" /><path d="M4 10H17C18.6569 10 20 11.3431 20 13C20 14.6569 18.6569 16 17 16H9" /><path d="M12 13L9 16L12 19" /><path d="M4 20H7" /></g></svg>);
  }
  return (<svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><path fill="none" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.5" d="M7 3.5l5 4.5 5-4.5M7 20.5l5-4.5 5 4.5M5 12h14" /></svg>);
}

/// Rust: `Changes::header_toggle`. A latched `active` holds the hover wash
/// and the full text tone, so the pane says which layout it is in without a
/// label. `header_button` is the same element with `active` false.
export function headerToggle(label: string, icon: 'split' | 'wrap' | 'fold', active: boolean,
  tooltip: string | null, onClick: () => void, onHover: (shown: boolean) => void) {
  return (
    <div className="relative flex-none">
      <button type="button" aria-label={label} aria-pressed={active} data-control={label}
        onClick={onClick} onMouseEnter={() => onHover(true)} onMouseLeave={() => onHover(false)}
        onFocus={() => onHover(true)} onBlur={() => onHover(false)}
        className={active
          ? 'size-6 flex-none flex items-center justify-center rounded-md cursor-pointer bg-wash/14 text-text hover:bg-wash/14 active:bg-wash/14 focus:bg-wash/14'
          : 'size-6 flex-none flex items-center justify-center rounded-md cursor-pointer text-text-muted/88 hover:bg-wash/14 active:bg-wash/14 focus:bg-wash/14'}>
        <div className="size-3.5">{toolbarIcon(icon)}</div>
      </button>
      {tooltip !== null && <div className="absolute top-7 right-0"><DiffHeaderTooltip label={tooltip} /></div>}
    </div>
  );
}

/// Rust: `Changes::render_scope_menu`. The dropdown the scope trigger opens.
function scopeMenu(scope: DiffScope, onSelectScope: (scope: DiffScope) => void) {
  return (
    <div className="absolute top-8 left-0 w-48 flex flex-col gap-0.5 p-1 rounded-xl border border-border-strong bg-surface-overlay">
      {SCOPE_MENU.map(entry => (
        <button type="button" key={entry} data-scope={entry} onClick={() => onSelectScope(entry)}
          className={entry === scope
            ? 'flex flex-row items-center gap-2.5 whitespace-nowrap px-2 py-1.5 rounded-lg text-left text-ui-13 text-text bg-element-active cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover'
            : 'flex flex-row items-center gap-2.5 whitespace-nowrap px-2 py-1.5 rounded-lg text-left text-ui-13 text-text/88 cursor-pointer hover:bg-element-hover active:bg-element-active focus:bg-element-hover'}>
          {scopeMenuLabel(entry)}
        </button>
      ))}
    </div>
  );
}

export type HeaderControlsProps = {
  scope: DiffScope;
  mode: DiffMode;
  wrapLines: boolean;
  allCollapsed: boolean;
  scopeMenuOpen: boolean;
  /// Rust: wrap_toggle's `.tooltip(...)`. Only that control has one.
  wrapTooltip: string | null;
  onToggleScopeMenu: () => void;
  onSelectScope: (scope: DiffScope) => void;
  onToggleMode: () => void;
  onToggleWrap: () => void;
  onToggleCollapseAll: () => void;
  onHoverControl: (label: string | null) => void;
};

/// Rust: `Changes::render_header_controls`, the scoped (non commit-pinned)
/// arm: scope dropdown, then the split, wrap and fold-all controls.
export function renderHeaderControls(props: HeaderControlsProps) {
  return (
    <div className="flex-none h-8 flex flex-row items-center gap-2 px-4 border-b border-border">
      <div className="relative flex-none">
        <button type="button" data-control="scope" aria-expanded={props.scopeMenuOpen}
          onClick={props.onToggleScopeMenu}
          className="h-6 px-2 flex-none flex flex-row items-center gap-1.5 rounded-md text-ui-12 text-text whitespace-nowrap cursor-pointer bg-wash/5 hover:bg-wash/14 active:bg-wash/14 focus:bg-wash/14">
          {scopeMenuLabel(props.scope)}
          <div className="size-3 text-text-muted/88">
            <svg viewBox="0 0 24 24" className="size-full" aria-hidden="true"><path fill="none" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.5" d="m19 9l-7 6l-7-6" /></svg>
          </div>
        </button>
        {props.scopeMenuOpen && scopeMenu(props.scope, props.onSelectScope)}
      </div>
      <div className="flex-1" />
      <div className="flex-none flex flex-row items-center gap-1">
        {headerToggle('Side by side', 'split', props.mode === 'split', null, props.onToggleMode, () => undefined)}
        {headerToggle('Wrap long lines', 'wrap', props.wrapLines, props.wrapTooltip, props.onToggleWrap, shown => props.onHoverControl(shown ? 'Wrap long lines' : null))}
        {headerToggle('Fold all files', 'fold', props.allCollapsed, null, props.onToggleCollapseAll, () => undefined)}
      </div>
    </div>
  );
}
