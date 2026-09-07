// Rust: app/crates/ui/src/files/notice.rs (Tone, refused_text, prompt_text, notice).
import type { ReactNode } from 'react';

// What the notice is about; only the edge colour differs.
export type NoticeTone = 'refused' | 'ask';

// The save-refused message: lead and detail.
export function refusedText(reason: string): readonly [string, string] {
  return ['Save refused.', reason];
}

// The unsaved-edits message: lead and detail. The file being left is already
// in the header above, so the detail leads with the choice.
export function promptText(here: string, next: string): readonly [string, string] {
  return ['Unsaved changes.', `Save or discard ${here} before opening ${next}?`];
}

// The message on its own line, the actions on the line below, wrapping if even
// they do not fit. The notice sits on comet's raised surface with its body
// text and says its tone with an edge on the left.
export function notice(tone: NoticeTone, lead: string, detail: string, actions: ReactNode) {
  return (
    <div role={tone === 'refused' ? 'alert' : 'group'} data-notice={tone}
      className={tone === 'refused'
        ? 'flex flex-col gap-1.5 px-2.5 py-2 bg-surface-raised border-l border-danger text-ui-12 text-text'
        : 'flex flex-col gap-1.5 px-2.5 py-2 bg-surface-raised border-l border-warning text-ui-12 text-text'}>
      <div className="flex items-start gap-1.5 min-w-0">
        <span className="flex-none font-medium">{lead}</span>
        <span className="flex-1 min-w-0 truncate whitespace-nowrap">{detail}</span>
      </div>
      <div className="flex flex-wrap items-center justify-end gap-2">{actions}</div>
    </div>
  );
}
