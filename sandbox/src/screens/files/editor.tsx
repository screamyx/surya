// Rust: app/crates/ui/src/files/editor.rs (FileEditor::render, header, banner,
// prompt, placeholder) and files/editor_doc.rs (Body, Conflict, EditorDoc).
import { btnDanger, btnGhost, btnPrimary } from './buttons';
import { notice, promptText, refusedText } from './notice';

// files/editor_doc.rs Body: what the editor shows for a path.
export type EditorBody =
  | { kind: 'empty' }
  | { kind: 'loading' }
  | { kind: 'text'; text: string }
  | { kind: 'binary'; size: number }
  | { kind: 'tooLarge'; size: number; max: number }
  | { kind: 'error'; why: string };

// files/editor_doc.rs EditorDoc, the parts the render reads. The buffer is a
// read-only paint here: text editing stays in Rust, so `dirty` and `line` are
// fixture facts rather than buffer comparisons.
export type EditorDocument = {
  body: EditorBody;
  dirty: boolean;
  saving: boolean;
  note: string | null;
  conflict: { reason: string } | null;
  line: number;
};

export type FileEditorProps = {
  path: string | null;
  doc: EditorDocument;
  // The path waiting behind the unsaved-edits prompt.
  pendingOpen: string | null;
  onSave: () => void;
  onKeepEditing: () => void;
  onDiscard: () => void;
  onReloadFromDisk: () => void;
  onOverwrite: () => void;
};

// files/editor.rs FileEditor::header.
function editorHeader({ path, doc }: FileEditorProps) {
  return (
    <div className="flex flex-none items-center gap-2 h-7 px-2.5 border-b border-border text-ui-12 text-text-muted">
      <span className="min-w-0 truncate whitespace-nowrap text-text">{path === null ? 'No file' : path}</span>
      {doc.dirty ? <span aria-label="unsaved edits" className="flex-none text-warning">●</span> : null}
      {doc.saving ? <span className="flex-none">saving…</span> : null}
      <span className="flex-1" />
      {doc.body.kind === 'text' ? <span className="flex-none">Ln {doc.line}</span> : null}
      {doc.note === null ? null : <span className="flex-none truncate text-danger">{doc.note}</span>}
    </div>
  );
}

// files/editor.rs FileEditor::placeholder.
function placeholderText(body: EditorBody): string | null {
  if (body.kind === 'text') return null;
  if (body.kind === 'empty') return 'Select a file in the tree';
  if (body.kind === 'loading') return 'Loading…';
  if (body.kind === 'binary') return `Binary file, ${body.size} bytes. No text view.`;
  if (body.kind === 'tooLarge') return `${body.size} bytes is over the ${body.max} byte read cap.`;
  return `Could not open: ${body.why}`;
}

// files/editor.rs FileEditor::prompt. Three ways out, all explicit.
function editorPrompt(props: FileEditorProps) {
  if (props.pendingOpen === null || !props.doc.dirty) return null;
  const [lead, detail] = promptText(props.path === null ? '' : props.path, props.pendingOpen);
  return notice('ask', lead, detail, [
    btnGhost('Keep editing', 'files-editor-keep', props.onKeepEditing),
    btnDanger('Discard', 'files-editor-discard', props.onDiscard),
    btnPrimary('Save', 'files-editor-save-then-open', props.onSave),
  ]);
}

// files/editor.rs FileEditor::banner. Both ways out lose something, so the one
// that destroys what is on disk wears the danger variant.
function editorBanner(props: FileEditorProps) {
  if (props.doc.conflict === null) return null;
  const [lead, detail] = refusedText(props.doc.conflict.reason);
  return notice('refused', lead, detail, [
    btnGhost('Reload from disk', 'files-editor-reload', props.onReloadFromDisk),
    btnDanger('Overwrite', 'files-editor-overwrite', props.onOverwrite),
  ]);
}

// A run of spaces collapses under every whitespace class the whitelist admits,
// which flattens a blank line to nothing and strips a line's indentation. The
// buffer's spaces are drawn as non-breaking spaces so the columns hold. GPUI
// lays the original text out and needs none of this.
function codeLine(line: string): string {
  const nbsp = '\u00A0';
  return line === '' ? nbsp : line.replaceAll(' ', nbsp);
}

export function FileEditor(props: FileEditorProps) {
  const placeholder = placeholderText(props.doc.body);
  return (
    <div data-pane="files-editor" className="flex flex-col size-full bg-surface">
      {editorHeader(props)}
      {editorPrompt(props)}
      {editorBanner(props)}
      {placeholder === null ? null : (
        <div className="flex-1 flex items-center justify-center text-ui-13 text-text-faint">{placeholder}</div>
      )}
      {props.doc.body.kind === 'text' ? (
        <div className="flex-1 min-h-0 overflow-y-scroll p-2 text-ui-13 text-text">
          {props.doc.body.text.split('\n').map((line, index) => (
            <div key={index} className="whitespace-nowrap">{codeLine(line)}</div>
          ))}
        </div>
      ) : null}
    </div>
  );
}
