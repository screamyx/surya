import { readFileSync, writeFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import { createHash } from 'node:crypto';
import { motionClasses, motionSources, translatingMotionClasses } from './motion.mjs';

const root = resolve(import.meta.dirname, '..');
const checkout = resolve(process.argv[2] ?? '/tmp/surya-sandbox-gpui');
const revision = 'a07e9577ec788feb73c06fe7e307a3df8adaa895';
if (execFileSync('git', ['-C', checkout, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim() !== revision) {
  throw new Error(`Expected GPUI ${revision}`);
}
const paths = ['crates/gpui/src/styled.rs', 'crates/gpui_macros/src/styles.rs',
  'crates/gpui/src/elements/div.rs', 'crates/gpui/src/geometry.rs', 'crates/gpui/src/style.rs'];
const sources = paths.map(path => readFileSync(resolve(checkout, path), 'utf8'));
const [styled, macros, div] = sources;
const methods = new Set([...`${styled}\n${macros}`.matchAll(/\bfn (\w+)/g)].map(m => m[1]));
const macroPrefixes = new Set([...macros.matchAll(/prefix: "(\w+)"/g)].map(m => m[1]));
const defaultLineHeight = Number(sources[3].match(/pub const fn phi\(\)[\s\S]*?relative\(([\d._]+)\)/)[1].replaceAll('_', ''));
if (!sources[4].includes('line_height: phi()')) throw new Error('GPUI default line height changed');
const exact = {};
const families = [];
function add(cls, method, args = '') {
  if (!methods.has(method) && !macroPrefixes.has(method)) throw new Error(`No method: ${method}`);
  exact[cls] = `.${method}(${args})`;
}
// Every direct helper below is checked against the actual trait/macro source.
for (const name of ['flex', 'flex_col', 'flex_row', 'flex_col_reverse', 'flex_row_reverse',
  'flex_1', 'flex_auto', 'flex_initial', 'flex_none', 'flex_wrap', 'flex_nowrap',
  'items_start', 'items_end', 'items_center', 'items_baseline', 'items_stretch',
  'justify_start', 'justify_end', 'justify_center', 'justify_between', 'justify_around',
  'justify_evenly', 'self_center', 'self_stretch', 'relative', 'absolute', 'hidden',
  'visible', 'invisible', 'overflow_hidden', 'overflow_x_hidden', 'overflow_y_hidden',
  'truncate', 'whitespace_nowrap', 'whitespace_normal', 'text_left', 'text_center',
  'text_right', 'cursor_pointer', 'cursor_default', 'italic', 'not_italic']) {
  add(name.replaceAll('_', '-'), name);
}
add('shrink-0', 'flex_shrink_0');
add('grow', 'flex_grow_1');
add('grow-0', 'flex_grow_0');
// Scroll helpers belong to StatefulInteractiveElement, not Styled.
for (const axis of ['x', 'y']) {
  const method = `overflow_${axis}_scroll`;
  if (!div.includes(`fn ${method}(`)) throw new Error(`Missing ${method}`);
  exact[`overflow-${axis}-scroll`] = `.id("stable-id").${method}()`;
}
const spacing = ['0', '0.25', '0.5', '0.75', '1', '1.5', '2', '2.25', '2.5', '3',
  '3.5', '3.75', '4', '5', '6', '7', '7.5', '8', '9', '9.5', '10', '11', '12',
  '14', '16', '20', '24', '28', '32', '40', '48', '55', '64', '80', '96', '184'];
for (const prefix of ['p', 'px', 'py', 'pt', 'pb', 'pl', 'pr', 'm', 'mx', 'my', 'mt',
  'mb', 'ml', 'mr', 'w', 'h', 'size', 'min_w', 'min_h', 'max_w', 'max_h', 'gap',
  'gap_x', 'gap_y', 'top', 'bottom', 'left', 'right']) {
  if (!macroPrefixes.has(prefix)) throw new Error(`No macro prefix ${prefix}`);
  families.push({ prefix: `${prefix.replaceAll('_', '-')}-`, suffixes: spacing,
    gpui: `.${prefix}(rems(Number(suffix) / 4))` });
}
for (const prefix of ['w', 'h', 'size', 'min_w', 'min_h', 'max_w', 'max_h']) {
  add(`${prefix.replaceAll('_', '-')}-full`, prefix, 'relative(1.0)');
}
for (const prefix of ['m', 'mx', 'my', 'mt', 'mb', 'ml', 'mr', 'w', 'h']) {
  add(`${prefix}-auto`, prefix, 'auto()');
}
for (const [name, n] of Object.entries({ none: 0, sm: 0.25, md: 0.375, lg: 0.5, xl: 0.75 })) {
  add(`rounded-${name}`, 'rounded', `rems(${n})`);
}
add('rounded-full', 'rounded', 'px(9999.0)');
// Border helper names are generated from border prefixes and suffixes.
for (const side of ['', '_t', '_b', '_l', '_r']) {
  const method = `border${side}`;
  if (!macroPrefixes.has(method)) throw new Error(`No border prefix ${method}`);
  exact[`border${side.replace('_', '-')}`] = `.${method}_1()`;
}
for (const [weight, name] of Object.entries({ normal: 'NORMAL', medium: 'MEDIUM', semibold: 'SEMIBOLD', bold: 'BOLD' })) {
  add(`font-${weight}`, 'font_weight', `FontWeight::${name}`);
}
add('font-sans', 'font_family', 'theme.font_sans.clone()');
add('font-mono', 'font_family', 'theme.font_mono.clone()');
add('leading-normal', 'line_height', 'relative(1.5)');
add('leading-gpui', 'line_height', 'phi()');
add('leading-none', 'line_height', 'relative(1.0)');
for (const px of [10, 11, 12, 13, 14, 16, 20]) add(`text-ui-${px}`, 'text_size', `ui_rems(${px}.0)`);
for (const n of [0, 50, 55, 100]) add(`opacity-${n}`, 'opacity', `${n / 100}`);
// Motion. These classes are the only admitted animation vocabulary, and they
// come from the surya catalog rather than from gpui: gpui has no transition or
// keyframe API, so the pair for each one is a `motion.rs` helper or the tween
// its call site runs. scripts/motion.mjs reads every duration, curve and
// endpoint out of the Rust so a designer cannot dial a value gpui cannot paint.
Object.assign(exact, motionClasses());
// A group marker for the group-hover / group-active variants below.
if (!div.includes('fn group(')) throw new Error('Missing group');
exact.group = '.group("stable-id")';
const theme = readFileSync(resolve(root, '../app/crates/ui/src/theme.rs'), 'utf8');
const struct = theme.split('pub struct Theme {')[1].split('\n}')[0];
const colors = [...struct.matchAll(/pub (\w+): Hsla/g)].map(m => m[1].replaceAll('_', '-'));
// wash is the native appearance-dependent black/white helper, used by Shell.
colors.push('wash', 'claude-brand');
// Which tokens may carry an alpha modifier is derived, not listed. Tailwind
// multiplies a token's existing alpha and gpui `opacity()` replaces it, so the
// two only agree where the token is already opaque. src/theme.ts holds the
// resolved value of every token in both appearances, so read the alpha there.
// text_dim passed this gate as a hand-written name for months after #183 folded
// it out of the Rust theme: a class that linted clean and painted nothing.
const resolved = JSON.parse(readFileSync(resolve(root, 'src/theme.ts'), 'utf8')
  .match(/export const themes = ([\s\S]*?) as const;/)[1]);
// Theme.syntax is a SyntaxPalette rather than an Hsla, so it is not a Theme
// field and does not reach the sweep above. It is paint-only and every call
// site reads it through .text_color(), so it joins the text- family alone.
const syntaxFields = [...theme.split('pub struct SyntaxPalette {')[1].split('\n}')[0]
  .matchAll(/pub (\w+): Hsla/g)].map(m => `syntax-${m[1].replaceAll('_', '-')}`);
// Theme.terminal is a TerminalColors, and its ansi member is an array, so it is
// invisible to the sweep for the same reason. terminal::view::resolve_color
// feeds a cell's foreground AND its background from the same value, so the
// palette joins both families; the selection is only ever a fill quad
// (view.rs:503), so it is a background alone. Nothing paints a terminal border.
const terminalFields = [...theme.split('pub struct TerminalColors {')[1].split('\n}')[0]
  .matchAll(/pub (\w+): (?:Hsla|\[Hsla; (\d+)\])/g)].flatMap(([, field, arity]) =>
    arity ? Array.from({ length: Number(arity) }, (_, i) => `terminal-${field}-${i}`)
      : [`terminal-${field.replaceAll('_', '-')}`]);
const terminalPaint = terminalFields.filter(name => name !== 'terminal-selection');
const alphaSuffixes = Object.keys(resolved.dark)
  .filter(key => ['dark', 'light'].every(mode => /\/ 1\)$/.test(resolved[mode][key])))
  .map(key => key.replaceAll('_', '-'));
for (const suffix of alphaSuffixes) {
  if (![...colors, ...syntaxFields, ...terminalFields].includes(suffix)) {
    throw new Error(`Opaque token ${suffix} has no Rust field`);
  }
}
for (const [prefix, method] of [['bg-', 'bg'], ['text-', 'text_color'], ['border-', 'border_color']]) {
  if (!methods.has(method)) throw new Error(`Missing ${method}`);
  const suffixes = [...colors,
    ...(prefix === 'text-' ? syntaxFields : []),
    ...(prefix === 'bg-' ? terminalFields : prefix === 'text-' ? terminalPaint : [])];
  families.push({ prefix, suffixes, alpha: [5, 10, 14, 50, 88],
    alphaSuffixes: suffixes.filter(suffix => alphaSuffixes.includes(suffix)),
    gpui: `.${method}(theme.<suffix_underscored>)`,
    alphaGpui: `.${method}(theme.<suffix_underscored>.opacity(alpha / 100))`,
    note: 'wash maps to crate::theme::wash(alpha); claude-brand maps to crate::icons::claude_brand(); '
      + 'a syntax- suffix maps to theme.syntax.<field>, the color SyntaxPalette::color() returns for that HighlightKind; '
      + 'a terminal-ansi-N suffix maps to theme.terminal.ansi[N], what terminal::view::resolve_color returns for CellColor::Indexed(N)'  });
}
const variants = {};
// Each variant is one interaction style method read in div.rs. `focus-within:`
// is gpui's `in_focus`, which applies when the tracked handle's subtree holds
// focus (`focus_handle.within_focused`). gpui has no disabled style, so there
// is no `disabled:` here.
for (const [variant, method, suffix] of [
  ['hover', 'hover', ''],
  ['active', 'active', ''],
  ['focus', 'focus', ' with .track_focus(&handle)'],
  ['focus-visible', 'focus_visible', ' with .track_focus(&handle)'],
  ['focus-within', 'in_focus', ' with .track_focus(&handle)'],
]) {
  if (!div.includes(`fn ${method}(`)) throw new Error(`Missing state ${method}`);
  variants[`${variant}:`] = `.${method}(|s| s.<mapped_style>)${suffix}`;
}
for (const [variant, method] of [['group-hover', 'group_hover'], ['group-active', 'group_active']]) {
  if (!div.includes(`fn ${method}(`)) throw new Error(`Missing state ${method}`);
  variants[`${variant}:`] = `.${method}("stable-id", |s| s.<mapped_style>) with .group("stable-id") on the ancestor`;
}
const out = { revision, defaultLineHeight,
  sources: [...paths.map((path, i) => ({ origin: 'gpui', path,
    sha256: createHash('sha256').update(sources[i]).digest('hex') })), ...motionSources],
  exact, families, variants, motionNeedsPosition: translatingMotionClasses,
  reactImports: ['useState'], reactTypeImports: ['ReactNode'],
  notes: ['Finite subset, not every fork capability.', 'No composed variants.',
    'Default Tailwind text sizes and shadows intentionally omitted.',
    'gpui source paths are relative to the gpui-surya checkout; surya ones to this repository.',
    'motion-* classes carry a fixed catalog timing. There is no free duration, delay or easing utility.',
    'A motion-* class that moves a relative inset needs relative or absolute beside it: gpui divs default to Position::Relative, CSS boxes to static.'] };
// Compact entries keep the contract readable and below the repository line cap.
const json = JSON.stringify(out, null, 2).replace(/\[\n\s+((?:"[^"]*",?\s*)+)\n\s*\]/g,
  (_, items) => `[${items.replace(/\s*\n\s*/g, ' ').trim()}]`);
writeFileSync(resolve(root, 'whitelist.json'), `${json}\n`);
console.log(`Generated ${Object.keys(exact).length} exact classes and ${families.length} bounded prefixes from GPUI ${revision}`);
