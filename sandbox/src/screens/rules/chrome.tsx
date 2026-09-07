// Rust: app/crates/ui/src/inbox/chrome.rs (row_card, button, body_text,
// command_text, empty_state).
//
// These five helpers are not yet in src/screens/chrome.tsx, which so far
// carries only badge and hint_chip. They live here so the rules screen can be
// ported without editing a shared file; promote them into screens/chrome.tsx
// when a second screen needs them.
import type { ReactNode } from 'react';

// chrome.rs:57, ButtonTone. How loud a button is.
export type ButtonTone = 'primary' | 'quiet' | 'danger';

// chrome.rs:174. The card a RULES row sits in.
export function rowCard(id: string, children: ReactNode) {
  return (
    <div key={id} id={id} className="flex flex-col gap-1.5 p-2.5 rounded-lg border border-border bg-surface-card">
      {children}
    </div>
  );
}

// chrome.rs:68. A small action button. The caller supplies the id, the
// accessible name and the click, exactly as the Rust caller adds .id() and
// .on_click() to the returned Div.
export function button(tone: ButtonTone, label: string, action: { id: string; ariaLabel: string; onClick: () => void }) {
  return (
    <button type="button" id={action.id} aria-label={action.ariaLabel} onClick={action.onClick}
      className={tone === 'primary'
        ? 'flex-none px-2.25 py-1 rounded-md text-ui-12 font-medium cursor-pointer bg-solid text-on-solid hover:bg-solid/88'
        : tone === 'quiet'
          ? 'flex-none px-2.25 py-1 rounded-md text-ui-12 font-medium cursor-pointer border border-border-strong text-text-muted hover:bg-element-hover hover:text-text'
          : 'flex-none px-2.25 py-1 rounded-md text-ui-12 font-medium cursor-pointer border border-danger/50 text-danger hover:bg-danger/10'}>
      {label}
    </button>
  );
}

// chrome.rs:188. One line of secondary text.
export function bodyText(text: string) {
  return <div className="min-w-0 text-ui-12 text-text-muted">{text}</div>;
}

// chrome.rs:201. A monospace block for a tool command. It WRAPS rather than
// truncating: this is the literal text the user approved, and a command whose
// tail is cut off is a command they were not shown.
export function commandText(text: string) {
  return <code className="min-w-0 px-1.5 py-0.75 rounded-md bg-surface-raised text-ui-11 text-text whitespace-normal">{text}</code>;
}

// chrome.rs:141. The quiet state: a sentence, not a blank pane.
export function emptyState(text: string) {
  return <div className="flex flex-col items-center justify-center py-10 text-ui-13 text-text-faint">{text}</div>;
}
