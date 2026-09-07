// Rust: the quick_add block of app/crates/ui/src/tasks/board.rs::render_column
// plus app/crates/ui/src/tasks/quick_add.rs (enter submits, escape clears).
// The native box is a ComposerInput entity with the PaletteSearch key
// context; here it is a plain input, so the caret, IME and key contexts are
// not ported. The go button's hover(|s| s.opacity(0.9)) has no admitted step.
import { renderIcon } from '../../icons';

export function renderQuickAdd(
  text: string,
  onText: (value: string) => void,
  onSubmit: () => void,
) {
  return (
    <div className="flex flex-row items-center gap-2 mb-1">
      <div className="flex-1 min-w-0 overflow-hidden px-2.5 py-1.5 rounded-lg bg-input-bg border border-wash/10 text-ui-13">
        <input type="text" value={text} aria-label="Add a task" placeholder="Add a task"
          onChange={event => onText(event.target.value)}
          onKeyDown={event => {
            event.stopPropagation();
            if (event.key === 'Enter') { event.preventDefault(); onSubmit(); }
            if (event.key === 'Escape') { event.preventDefault(); onText(''); }
          }}
          className="w-full min-w-0 text-ui-13 text-text" />
      </div>
      <button type="button" aria-label="Add a task" onClick={onSubmit}
        className="flex-none w-7 h-7 rounded-lg bg-accent flex items-center justify-center cursor-pointer">
        <span className="size-3.5 text-on-accent">{renderIcon('plus')}</span>
      </button>
    </div>
  );
}
