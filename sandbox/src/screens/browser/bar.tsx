// Rust: app/crates/ui/src/browser_pane/bar.rs (row, zoom_reading, button).
// Render helpers on the same pane, not second stateful components.
import { renderIcon } from '../../icons';
import { plainTooltip } from './tabs';
import { zoomLabel, ZOOM_DEFAULT } from './state';

// bar.rs BAR_HEIGHT 36px is h-9. The field is 26px tall in Rust, which has no
// admitted step; h-7 (28px) is the nearest and keeps the field centred.
export type BarProps = {
  url: string;
  canBack: boolean;
  canForward: boolean;
  loading: boolean;
  zoomPercent: number;
  urlFocused: boolean;
  hovered: string | null;
  onHover: (id: string | null) => void;
  onUrlChange: (text: string) => void;
  onUrlFocus: (focused: boolean) => void;
  onSubmitUrl: () => void;
  onRestoreUrl: () => void;
  onBack: () => void;
  onForward: () => void;
  onReloadOrStop: () => void;
  onZoom: (percent: number) => void;
};

export function row(props: BarProps) {
  return (
    <div className="flex-none h-9 px-2 gap-1 flex flex-row items-center border-b border-border">
      {button('browser-back', renderIcon('arrow-left'), 'Back', props.canBack, props.hovered, props.onHover, props.onBack)}
      {button('browser-forward', renderIcon('arrow-right'), 'Forward', props.canForward, props.hovered, props.onHover, props.onForward)}
      {button('browser-reload', props.loading ? renderIcon('close') : renderIcon('refresh'),
        props.loading ? 'Stop' : 'Reload', true, props.hovered, props.onHover, props.onReloadOrStop)}
      {field(props)}
    </div>
  );
}

// comet's own input, wrapped in the border and background a settings text
// field gets. The border goes strong while the keyboard is in it.
function field(props: BarProps) {
  return (
    <div className={props.urlFocused
      ? 'flex-1 min-w-0 h-7 px-2.5 rounded-lg border border-border-strong bg-input-bg flex flex-row items-center gap-1.5 text-ui-13'
      : 'flex-1 min-w-0 h-7 px-2.5 rounded-lg border border-border bg-input-bg flex flex-row items-center gap-1.5 text-ui-13'}>
      <input type="text" aria-label="Address" data-field="browser-url" value={props.url}
        placeholder="Search or type an address"
        onChange={event => props.onUrlChange(event.target.value)}
        onFocus={() => props.onUrlFocus(true)} onBlur={() => props.onUrlFocus(false)}
        onKeyDown={event => {
          // keys.rs on_key: enter and escape are unbound in the field's own
          // context, so they reach the pane.
          if (event.key === 'Enter') props.onSubmitUrl();
          if (event.key === 'Escape') props.onRestoreUrl();
        }}
        className="flex-1 min-w-0 h-full text-ui-13 text-text" />
      {zoomReading(props.zoomPercent, props.onZoom)}
    </div>
  );
}

// The zoom, inside the field's right edge, and only when it is not 100%.
// Clicking it puts the page back to 100, as Chrome's own reading does.
function zoomReading(percent: number, onZoom: (percent: number) => void) {
  const label = zoomLabel(percent);
  if (label === null) return null;
  return (
    <button type="button" data-zoom="1" aria-label={`Reset zoom, now ${label}`}
      onClick={() => onZoom(ZOOM_DEFAULT)}
      className="flex-none h-5 px-1.5 flex items-center rounded-sm cursor-pointer bg-element-hover text-ui-11 text-text-muted hover:bg-element-active focus:bg-element-active">{label}</button>
  );
}

function button(id: string, glyph: ReturnType<typeof renderIcon>, label: string, enabled: boolean,
  hovered: string | null, onHover: (id: string | null) => void, onPress: () => void) {
  // A disabled history button keeps its place and goes quiet, as in Chrome.
  if (!enabled) {
    return (
      <span key={id} data-button={id} aria-disabled="true" aria-label={label}
        className="size-6 flex-none flex items-center justify-center rounded-md text-text-muted/50">
        <span className="size-3.5">{glyph}</span>
      </span>
    );
  }
  return (
    <span key={id} className="relative flex-none flex">
      <button type="button" data-button={id} aria-label={label} onClick={onPress}
        onMouseEnter={() => onHover(id)} onMouseLeave={() => onHover(null)}
        className="size-6 flex-none flex items-center justify-center rounded-md cursor-pointer text-text hover:bg-element-hover focus:bg-element-hover">
        <span className="size-3.5">{glyph}</span>
      </button>
      {hovered === id && <span className="absolute top-7 left-0">{plainTooltip(label)}</span>}
    </span>
  );
}
