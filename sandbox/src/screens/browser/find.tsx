// Rust: app/crates/ui/src/browser_pane/find.rs (bar, step).
// Render helpers on the same pane, not second stateful components.
import { renderIcon } from '../../icons';
import { plainTooltip } from './tabs';
import { findLabel } from './state';

// find.rs FIELD_WIDTH 172px has no admitted step; w-40 (160px) is the nearest
// and still leaves the strip room to shrink beside it.
export type FindProps = {
  query: string;
  result: { current: number; total: number } | null;
  focused: boolean;
  hovered: string | null;
  onHover: (id: string | null) => void;
  onQueryChange: (text: string) => void;
  onFindFocus: (focused: boolean) => void;
  onStepFind: (forward: boolean) => void;
  onCloseFind: () => void;
};

export function bar(props: FindProps) {
  const count = props.result === null ? null : findLabel(props.result.current, props.result.total);
  return (
    <div className="flex-none flex flex-row items-center gap-0.5">
      <div className={props.focused
        ? 'w-40 flex-none h-6 px-2 rounded-md border border-border-strong bg-input-bg flex flex-row items-center gap-1.5 text-ui-12'
        : 'w-40 flex-none h-6 px-2 rounded-md border border-border bg-input-bg flex flex-row items-center gap-1.5 text-ui-12'}>
        <span className="size-3 flex-none text-text-muted">{renderIcon('magnifer')}</span>
        <input type="text" aria-label="Find in page" data-field="browser-find" value={props.query}
          placeholder="Find in page"
          onChange={event => props.onQueryChange(event.target.value)}
          onFocus={() => props.onFindFocus(true)} onBlur={() => props.onFindFocus(false)}
          onKeyDown={event => {
            // keys.rs on_key: enter steps the run, shift-enter steps back,
            // escape closes the bar.
            if (event.key === 'Enter') props.onStepFind(!event.shiftKey);
            if (event.key === 'Escape') props.onCloseFind();
          }}
          className="flex-1 min-w-0 h-full text-ui-12 text-text" />
        {count !== null && <span className="flex-none text-ui-11 text-text-muted">{count}</span>}
      </div>
      {step('browser-find-prev', renderIcon('arrow-up'), 'Previous match', props, false)}
      {step('browser-find-next', renderIcon('arrow-down'), 'Next match', props, true)}
      {iconButton('browser-find-close', renderIcon('close'), 'Close find', props, props.onCloseFind)}
    </div>
  );
}

function step(id: string, glyph: ReturnType<typeof renderIcon>, label: string, props: FindProps, forward: boolean) {
  return iconButton(id, glyph, label, props, () => props.onStepFind(forward));
}

function iconButton(id: string, glyph: ReturnType<typeof renderIcon>, label: string, props: FindProps, onPress: () => void) {
  return (
    <span key={id} className="relative flex-none flex">
      <button type="button" data-button={id} aria-label={label} onClick={onPress}
        onMouseEnter={() => props.onHover(id)} onMouseLeave={() => props.onHover(null)}
        className="size-5 flex-none flex items-center justify-center rounded-sm cursor-pointer text-text-muted hover:bg-element-hover focus:bg-element-hover">
        <span className="size-2.5">{glyph}</span>
      </button>
      {props.hovered === id && <span className="absolute top-6 right-0">{plainTooltip(label)}</span>}
    </span>
  );
}
