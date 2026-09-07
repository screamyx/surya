// Fixture props only. The native engine continues to own the PTY stream, the
// emulator and every terminal RPC (app/crates/ui/src/terminal/panel.rs).
// Colours are CellColor slots, the way view.rs:99 resolve_color reads them:
// 6 cyan for the prompt path, 2 green and 3 yellow for cargo's bold labels,
// 1 red for a failure, 8 for the SGR 90 the exit notice writes.
// Column alignment uses no-break spaces (\u00a0): the whitelist has no
// whitespace-pre, so runs of ordinary spaces collapse in the browser.
import type { TerminalTab } from '../screens/Terminal';

const gap = '\u00a0\u00a0\u00a0\u00a0';

export const seededTerminalTabs: readonly TerminalTab[] = [
  {
    key: 'term-1',
    // Emulator::title(), the shell's own OSC title, wins over "Terminal N".
    title: 'surya',
    exited: null,
    historyLines: 412,
    cursor: { row: 8, col: 14 },
    rows: [
      { id: 'r1', spans: [{ text: '~/git/surya', color: 6 }, { text: ' % cargo check --workspace --all-targets' }] },
      { id: 'r2', spans: [{ text: `${gap}Checking`, color: 2, bold: true }, { text: ' surya-ui v0.1.0' }] },
      { id: 'r3', spans: [{ text: `${gap}Checking`, color: 2, bold: true }, { text: ' surya-engine v0.1.0' }] },
      { id: 'r4', spans: [{ text: 'warning', color: 3, bold: true }, { text: ': unused variable: ' }, { text: '`window`', bold: true }] },
      { id: 'r5', spans: [{ text: `${gap}-->`, color: 4, bold: true }, { text: ' crates/ui/src/terminal/panel.rs:1641:24', dim: true }] },
      { id: 'r6', spans: [{ text: `${gap}Finished`, color: 2, bold: true }, { text: ' `dev` profile in 4.21s' }] },
      { id: 'r7', spans: [{ text: '~/git/surya', color: 6 }, { text: ' % ' }, { text: 'the emulator keeps its scrollback', italic: true }], selection: [14, 47] },
      { id: 'r8', spans: [{ text: '' }] },
      { id: 'r9', spans: [{ text: '~/git/surya', color: 6 }, { text: ' % ' }] },
    ],
  },
  {
    key: 'term-2',
    title: 'cargo test',
    exited: 1,
    historyLines: 0,
    cursor: null,
    rows: [
      { id: 'e1', spans: [{ text: 'test terminal::panel::tests::height_clamps_between_160_and_55vh ... ' }, { text: 'ok', color: 2 }] },
      { id: 'e2', spans: [{ text: 'test terminal::view::tests::cell_hit_anchors_to_the_nearer_edge ... ' }, { text: 'FAILED', color: 1, bold: true }] },
      { id: 'e3', spans: [{ text: '' }] },
      { id: 'e4', spans: [{ text: 'test result: ' }, { text: 'FAILED', color: 1, bold: true }, { text: '. 25 passed; 1 failed' }] },
      { id: 'e5', spans: [{ text: '' }] },
      // panel.rs::exit_message: "\r\n\x1b[90m[process exited N]\x1b[0m\r\n".
      { id: 'e6', spans: [{ text: '[process exited 1]', color: 8 }] },
    ],
  },
  {
    key: 'term-3',
    // panel.rs::open_tab's fallback title, before the shell sets an OSC title.
    title: 'Terminal 3',
    exited: null,
    historyLines: 0,
    cursor: { row: 0, col: 0 },
    rows: [{ id: 'n1', spans: [{ text: '' }] }],
  },
];

/// A chat whose terminal drawer has been opened but has no tab yet.
export const emptyTerminalTabs: readonly TerminalTab[] = [];
