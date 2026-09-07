// Rust: app/crates/ui/src/composer.rs, ComposerInput (line 3334), its Render
// impl (3207) and the ComposerTextElement paint (2902-3207).
//
// GPUI paints its own editor. A browser textarea has no admitted GPUI pair and
// `ref` is banned, so this component paints the SURFACE only: the shaped text
// runs, the mention chip washes (code-wash, the inline-code recipe), the
// selection fill (selection), the caret quad (caret, 2px wide) and the shaped
// ghost suffix. Text entry itself stays native.
import type { InputLine, InputSegment } from '../../fixtures/composer';

export type ComposerInputProps = {
  lines: readonly InputLine[];
  isPlaceholder: boolean;
  onHoverMention: (path: string | null) => void;
};

function renderSegment(
  segment: InputSegment,
  index: number,
  isPlaceholder: boolean,
  onHoverMention: (path: string | null) => void,
) {
  if (segment.kind === 'caret') {
    // prepaint: fill(size(px(2.0), line_height), caret_color).
    return <span key={index} className="w-0.5 h-full flex-none bg-caret" />;
  }
  if (segment.kind === 'ghost') {
    // The completion preview shaped at the end-of-text caret.
    return <span key={index} className="whitespace-nowrap text-text-faint">{segment.text}</span>;
  }
  if (segment.kind === 'selected') {
    return <span key={index} className="whitespace-nowrap bg-selection text-text">{segment.text}</span>;
  }
  if (segment.kind === 'mention') {
    const path = segment.path;
    return (
      <span key={index} data-mention={path}
        onMouseEnter={() => onHoverMention(path)} onMouseLeave={() => onHoverMention(null)}
        className="h-5 flex items-center whitespace-nowrap px-0.5 rounded-sm bg-code-wash text-text cursor-default">
        {segment.text}
      </span>
    );
  }
  return (
    <span key={index} className={isPlaceholder
      ? 'whitespace-nowrap text-text-faint'
      : 'whitespace-nowrap text-text'}>{segment.text}</span>
  );
}

export function ComposerInput({ lines, isPlaceholder, onHoverMention }: ComposerInputProps) {
  return (
    <div data-input="composer" aria-label="Message" className="w-full min-w-0 flex flex-col text-ui-14 leading-gpui font-sans">
      {lines.map(line => (
        <div key={line.id} className="h-6 w-full min-w-0 flex flex-row items-center overflow-hidden">
          {line.segments.map((segment, index) =>
            renderSegment(segment, index, isPlaceholder, onHoverMention))}
        </div>
      ))}
    </div>
  );
}
