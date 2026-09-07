// Rust: app/crates/ui/src/markdown/render.rs (render_code_block,
// CodeBlockTooltip, the CodeScrollbarUi thumb and its drag ghost).
import { icon } from './icons';

/// Rust: the `CodeUi` / `CodeScrollbarUi` fields `Transcript::code_ui_for`
/// hands each block, reduced to what a designer can drive by hand. `fit` is
/// the Fit-content toggle, `hovered` is the block-level `viewport_hover` that
/// reveals the horizontal scrollbar, `copied` flips the Copy glyph to a check.
export type CodeState = {
  fit: (key: string) => boolean;
  hovered: (key: string) => boolean;
  copied: (key: string) => boolean;
  onToggleFit: (key: string) => void;
  onHoverCode: (key: string, hovered: boolean) => void;
  onCopyCode: (key: string, code: string) => void;
};

/// gpui shapes a code line exactly as it arrives; HTML collapses runs of
/// spaces, so leading indent and column alignment would vanish. The whitelist
/// admits no `whitespace-pre`, so the spaces are held open in the string
/// instead. Shared with the tool detail bodies, which are mono for the same
/// reason.
export function preserveSpaces(line: string) {
  return line
    .replace(/^ +/, run => ' '.repeat(run.length))
    .replace(/ {2,}/g, run => ' '.repeat(run.length));
}

/// Rust: `CodeBlockTooltip` - the only tooltip in the transcript, over the Fit
/// button. Driven from local hover state; gpui's `.tooltip()` has no admitted
/// pair and `group-hover` is banned here.
function tooltip(label: string) {
  return (
    <div className="absolute bottom-6 right-0 px-2 py-1.5 rounded-md border border-border-strong bg-surface-raised text-ui-11 text-text whitespace-nowrap">
      {label}
    </div>
  );
}

/// Rust: the code block's 28px header - the language name at the left, then
/// the Fit and Copy ghost buttons. Drawn whenever there is a language or an
/// action, which in the transcript is always.
function header(
  language: string | undefined,
  key: string,
  code: string,
  state: CodeState,
  fit: boolean,
) {
  const label = fit ? 'Use horizontal scrolling' : 'Fit content';
  return (
    <div className="h-7 pl-3 pr-1 border-b border-border bg-wash/5 flex flex-row items-center justify-between">
      <div className="min-w-0 truncate text-ui-11 text-text-muted">{language}</div>
      <div className="flex-none flex flex-row items-center gap-0.5">
        <div className="relative">
          <button type="button" aria-label={label} onClick={() => state.onToggleFit(key)}
            onMouseEnter={() => state.onHoverCode(`${key}-fit`, true)}
            onMouseLeave={() => state.onHoverCode(`${key}-fit`, false)}
            className={fit
              ? 'size-5 rounded-md flex items-center justify-center cursor-pointer text-text-muted bg-wash/10 motion-hover-fade hover:bg-wash/14 active:bg-element-active focus:bg-wash/14'
              : 'size-5 rounded-md flex items-center justify-center cursor-pointer text-text-muted motion-hover-fade hover:bg-wash/10 active:bg-element-active focus:bg-wash/10'}>
            <span className="size-3 flex items-center justify-center">{icon('wrap-text')}</span>
          </button>
          {state.hovered(`${key}-fit`) && tooltip(label)}
        </div>
        <button type="button" aria-label="Copy code" onClick={() => state.onCopyCode(key, code)}
          className="h-5 px-1.5 rounded-sm flex flex-row items-center gap-1 cursor-pointer text-ui-10 text-text-muted motion-hover-fade hover:bg-wash/10 active:bg-element-active focus:bg-wash/10">
          <span className="size-3 flex items-center justify-center">{state.copied(key) ? icon('check') : icon('copy')}</span>
          {state.copied(key) && <span>Copied</span>}
        </button>
      </div>
    </div>
  );
}

/// Rust: the 10px hit lane at the block's bottom edge with the rounded thumb.
/// Native derives the thumb's width and left from measured content; this
/// sandbox does not measure, so it draws the resting proportion.
function scrollbar() {
  return (
    <div className="absolute left-0 right-0 bottom-0 h-2.5">
      <div className="absolute left-1 bottom-0.5 w-20 h-1 rounded-full bg-text-faint/50" />
    </div>
  );
}

/// Rust: `render_code_block`. One bordered panel: header, then the lines in
/// mono at 12.5/18, either wrapped (Fit content) or on their own horizontal
/// scroller with the hover-revealed scrollbar over it.
export function renderCodeBlock(
  language: string | undefined,
  code: string,
  key: string,
  state: CodeState,
) {
  const fit = state.fit(key);
  const lines = code.split('\n');
  const body = (
    <div className={fit
      ? 'w-full min-w-0 px-3 py-2.5 flex flex-col text-ui-12 leading-normal whitespace-normal text-text'
      : 'min-w-full flex-none px-3 py-2.5 flex flex-col text-ui-12 leading-normal whitespace-nowrap text-text'}>
      {lines.map((line, li) => (
        <div key={li} className={fit ? 'w-full min-w-0' : 'h-4 flex-none'}>{preserveSpaces(line)}</div>
      ))}
    </div>
  );
  return (
    <div onMouseEnter={() => state.onHoverCode(key, true)} onMouseLeave={() => state.onHoverCode(key, false)}
      className="rounded-lg bg-wash/5 border border-border overflow-hidden relative">
      {header(language, key, code, state, fit)}
      <div className="w-full relative">
        {fit
          ? <div className="w-full min-w-0 overflow-hidden">{body}</div>
          : <div className="w-full min-w-0 flex overflow-x-scroll">{body}</div>}
        {!fit && state.hovered(key) && scrollbar()}
      </div>
    </div>
  );
}
