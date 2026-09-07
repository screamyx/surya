// Rust: app/crates/ui/src/files/mod.rs (FilesPane::render, and the TreeEvent
// subscription that hands a tree click to the editor).
import { useState } from 'react';
import type { EditorDocument } from './files/editor';
import { FileEditor } from './files/editor';
import type { FileEntry } from './files/model';
import { FileTreeView } from './files/tree';

export type FilesPaneProps = {
  entries: readonly FileEntry[];
  expandedSeed: readonly string[];
  // What FilesRead answered for each path. The engine still owns the read.
  documents: Readonly<Record<string, EditorDocument>>;
  onOpenFile: (path: string) => void;
  onSaveFile: (path: string) => void;
  onReloadFromDisk: (path: string) => void;
  onOverwrite: (path: string) => void;
};

const noFile: EditorDocument = {
  body: { kind: 'empty' }, dirty: false, saving: false, note: null, conflict: null, line: 1,
};

export function FilesPane(props: FilesPaneProps) {
  const [openPath, setOpenPath] = useState<string | null>(null);
  const [pendingOpen, setPendingOpen] = useState<string | null>(null);
  const opened = openPath === null ? undefined : props.documents[openPath];
  const doc = openPath === null ? noFile
    : opened === undefined
      ? { ...noFile, body: { kind: 'error' as const, why: 'the fixture has no read for this path' } }
      : opened;

  // files/editor_doc.rs EditorDoc::click. A dirty buffer is never replaced
  // without asking; the same dirty file clicked again takes the prompt down.
  function openFromTree(path: string) {
    if (!doc.dirty) {
      setPendingOpen(null);
      setOpenPath(path);
      props.onOpenFile(path);
      return;
    }
    if (path === openPath) setPendingOpen(null);
    else setPendingOpen(path);
  }

  // files/editor.rs save_then_open: the write goes, and with no save in
  // flight here the prompt steps aside rather than sit there doing nothing.
  function onSave() {
    if (openPath !== null) props.onSaveFile(openPath);
    setPendingOpen(null);
  }

  // files/editor.rs discard_then_open: the waiting path loads over the edits.
  function onDiscard() {
    if (pendingOpen === null) return;
    setOpenPath(pendingOpen);
    setPendingOpen(null);
    props.onOpenFile(pendingOpen);
  }

  return (
    <div data-pane="files" className="flex size-full bg-bg">
      <div className="flex-none w-64 h-full border-r border-border">
        <FileTreeView entries={props.entries} expandedSeed={props.expandedSeed} onOpen={openFromTree} />
      </div>
      <div className="flex-1 min-w-0 h-full">
        <FileEditor path={openPath} doc={doc} pendingOpen={pendingOpen}
          onSave={onSave} onKeepEditing={() => setPendingOpen(null)} onDiscard={onDiscard}
          onReloadFromDisk={() => { if (openPath !== null) props.onReloadFromDisk(openPath); }}
          onOverwrite={() => { if (openPath !== null) props.onOverwrite(openPath); }} />
      </div>
    </div>
  );
}
