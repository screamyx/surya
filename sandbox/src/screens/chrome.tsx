// Rust: app/crates/ui/src/inbox/chrome.rs.
// The shared inbox chrome, mirroring one Rust file. Three screens each grew a
// private copy of parts of this during the port; they are merged here so a
// chrome edit lands once and still points back at one native helper.
import type { ReactNode } from 'react';

/// The tone a state or a badge is drawn in. Rust: state_color(), kind_color().
export type Tone = 'accent' | 'warning' | 'danger' | 'busy' | 'success' | 'faint';

/// chrome.rs::dot. A round status dot, 6px.
export function dot(tone: Tone) {
  return <span aria-hidden="true" className={
    tone === 'accent' ? 'flex-none size-1.5 rounded-full bg-accent'
      : tone === 'warning' ? 'flex-none size-1.5 rounded-full bg-warning'
        : tone === 'danger' ? 'flex-none size-1.5 rounded-full bg-danger'
          : tone === 'busy' ? 'flex-none size-1.5 rounded-full bg-busy'
            : tone === 'success' ? 'flex-none size-1.5 rounded-full bg-success'
              : 'flex-none size-1.5 rounded-full bg-text-faint'} />;
}

/// chrome.rs::badge. The pill a row wears for its state, and the roll-up count
/// a parent shows for a blocked descendant.
export function badge(label: string, tone: Tone) {
  return <span className={
    tone === 'accent' ? 'flex-none px-1.5 py-0.25 rounded-full bg-accent/14 text-ui-10 font-medium text-accent'
      : tone === 'warning' ? 'flex-none px-1.5 py-0.25 rounded-full bg-warning/14 text-ui-10 font-medium text-warning'
        : tone === 'danger' ? 'flex-none px-1.5 py-0.25 rounded-full bg-danger/14 text-ui-10 font-medium text-danger'
          : tone === 'busy' ? 'flex-none px-1.5 py-0.25 rounded-full bg-busy/14 text-ui-10 font-medium text-busy'
            : tone === 'success' ? 'flex-none px-1.5 py-0.25 rounded-full bg-success/14 text-ui-10 font-medium text-success'
              : 'flex-none px-1.5 py-0.25 rounded-full bg-text-faint/14 text-ui-10 font-medium text-text-faint'}>{label}</span>;
}

/// chrome.rs::hint_chip. The quiet trailing hint on a collapsed row.
export function hintChip(label: string) {
  return <span className="flex-none px-1.5 py-1 rounded-md text-ui-11 text-text-faint">{label}</span>;
}

/// chrome.rs:141, empty_state. The quiet state: a sentence, not a blank pane.
export function emptyState(text: string) {
  return <div className="flex flex-col items-center justify-center py-10 text-ui-13 text-text-faint">{text}</div>;
}

/// chrome.rs:174, row_card. The card a rules row sits in.
export function rowCard(id: string, children: ReactNode) {
  return (
    <div key={id} id={id} className="flex flex-col gap-1.5 p-2.5 rounded-lg border border-border bg-surface-card">
      {children}
    </div>
  );
}

/// chrome.rs:57, ButtonTone. How loud a button is.
export type ButtonTone = 'primary' | 'quiet' | 'danger';

/// chrome.rs:68, button. The caller supplies the id, the accessible name and
/// the click, exactly as the Rust caller adds .id() and .on_click() to the Div.
export function button(tone: ButtonTone, label: string,
  action: { id: string; ariaLabel: string; onClick: () => void }) {
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

/// chrome.rs:188, body_text. One line of secondary text.
export function bodyText(text: string) {
  return <div className="min-w-0 text-ui-12 text-text-muted">{text}</div>;
}

/// chrome.rs:201, command_text. A block for a tool command. It WRAPS rather
/// than truncating: this is the literal text the user approved, and a command
/// whose tail is cut off is a command they were not shown.
export function commandText(text: string) {
  return <code className="min-w-0 px-1.5 py-0.75 rounded-md bg-surface-raised text-ui-11 text-text whitespace-normal">{text}</code>;
}
