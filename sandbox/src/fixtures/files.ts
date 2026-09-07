// Fixture props only. The native engine keeps FilesTree, FilesWatch,
// FilesSearch, FilesRead and FilesWrite. Shaped after app/crates/ui/src/
// files/demo.rs, which opens FilesPane on the surya checkout itself, so the
// sandbox and that demo show the same folders.
import type { EditorDocument } from '../screens/files/editor';
import type { FileEntry } from '../screens/files/model';

export const fileEntries: readonly FileEntry[] = [
  { path: 'AGENTS.md', kind: 'file', status: '', hasChildren: false },
  { path: 'README.md', kind: 'file', status: 'M', hasChildren: false },
  { path: 'app', kind: 'dir', status: '', hasChildren: false },
  { path: 'app/crates', kind: 'dir', status: '', hasChildren: false },
  { path: 'app/crates/browser', kind: 'dir', status: '', hasChildren: false },
  { path: 'app/crates/browser/Cargo.toml', kind: 'file', status: '', hasChildren: false },
  { path: 'app/crates/theme', kind: 'dir', status: '', hasChildren: false },
  { path: 'app/crates/theme/src', kind: 'dir', status: '', hasChildren: false },
  { path: 'app/crates/theme/src/builtins.rs', kind: 'file', status: '', hasChildren: false },
  { path: 'app/crates/theme/src/lib.rs', kind: 'file', status: 'M', hasChildren: false },
  { path: 'app/crates/ui', kind: 'dir', status: '', hasChildren: false },
  { path: 'app/crates/ui/assets', kind: 'dir', status: '', hasChildren: true },
  { path: 'app/crates/ui/src', kind: 'dir', status: '', hasChildren: false },
  { path: 'app/crates/ui/src/files', kind: 'dir', status: 'M', hasChildren: false },
  { path: 'app/crates/ui/src/files/editor.rs', kind: 'file', status: 'M', hasChildren: false },
  { path: 'app/crates/ui/src/files/mod.rs', kind: 'file', status: '', hasChildren: false },
  { path: 'app/crates/ui/src/files/notes.rs', kind: 'file', status: 'D', hasChildren: false },
  { path: 'app/crates/ui/src/files/tree.rs', kind: 'file', status: 'A', hasChildren: false },
  { path: 'app/crates/ui/src/theme.rs', kind: 'file', status: '', hasChildren: false },
  { path: 'deploy', kind: 'dir', status: '', hasChildren: true },
  { path: 'docs', kind: 'dir', status: '', hasChildren: false },
  { path: 'docs/decisions.md', kind: 'file', status: '', hasChildren: false },
  { path: 'docs/handoff-2026-09-05.md', kind: 'file', status: '?', hasChildren: false },
];

// The directories already listed and opened when the pane comes up. In Rust
// TreeModel starts empty and fills as each FilesTree answer lands.
export const expandedSeed: readonly string[] = [
  'app/crates', 'app/crates/ui', 'app/crates/ui/src', 'app/crates/ui/src/files', 'docs',
];

const clean = { dirty: false, saving: false, note: null, conflict: null, line: 1 };

// What FilesRead answered per path: one file for each Body the editor draws,
// plus the dirty and refused states the notices depend on.
export const fileDocuments: Readonly<Record<string, EditorDocument>> = {
  'AGENTS.md': {
    ...clean, line: 4,
    body: {
      kind: 'text',
      text: '# AGENTS.md\n\nGuidance for Claude Code, Codex, and any other agent working\nin this repository. CLAUDE.md is a one-line @AGENTS.md shim so\nevery tool reads this same file.\n\n## What surya is\n\nsurya is a fork of zeronsh/comet: a native GPUI desktop app in\nRust whose engine daemon runs Claude Code.\n',
    },
  },
  'README.md': {
    ...clean, dirty: true, line: 3,
    body: { kind: 'text', text: '# surya\n\nA native desktop app for running coding agents.\nBuild from app/ with cargo build -p surya.\n' },
  },
  'app/crates/browser/Cargo.toml': {
    ...clean, note: 'watch reconnecting', line: 2,
    body: { kind: 'text', text: '[package]\nname = "surya-browser"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\ncef = "151"\n' },
  },
  'app/crates/theme/src/builtins.rs': {
    ...clean, line: 6,
    body: { kind: 'text', text: 'fn variant(seed: Seeds, dark: bool) -> ThemeColors {\n    let background = seed.background;\n    let surface = mix(background, seed.accent, 0.04);\n    ThemeColors { background, surface }\n}\n' },
  },
  'app/crates/theme/src/lib.rs': {
    ...clean, saving: true, dirty: true, line: 12,
    body: { kind: 'text', text: 'pub mod builtins;\n\npub struct ThemeColors {\n    pub background: Hsla,\n    pub surface: Hsla,\n}\n' },
  },
  'app/crates/ui/src/files/editor.rs': {
    ...clean, dirty: true, line: 44,
    conflict: { reason: 'changed on disk since it was read' },
    body: { kind: 'text', text: 'impl FileEditor {\n    pub fn is_dirty(&self, cx: &gpui::App) -> bool {\n        self.doc.is_dirty(self.input.read(cx).text())\n    }\n}\n' },
  },
  'app/crates/ui/src/files/mod.rs': {
    ...clean, line: 1,
    body: { kind: 'text', text: 'pub mod demo;\npub mod editor;\npub mod editor_doc;\npub mod model;\npub mod notice;\npub mod tree;\n' },
  },
  'app/crates/ui/src/files/notes.rs': { ...clean, body: { kind: 'error', why: 'no such file or directory' } },
  'app/crates/ui/src/files/tree.rs': {
    ...clean, line: 9,
    body: { kind: 'text', text: 'pub enum TreeEvent {\n    Open(String),\n}\n' },
  },
  'app/crates/ui/src/theme.rs': { ...clean, body: { kind: 'loading' } },
  'docs/decisions.md': { ...clean, body: { kind: 'tooLarge', size: 2411520, max: 1048576 } },
  'docs/handoff-2026-09-05.md': { ...clean, body: { kind: 'binary', size: 918294 } },
};
