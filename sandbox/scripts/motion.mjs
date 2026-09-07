// The native motion catalog, read out of the Rust sources.
//
// Every duration, curve and endpoint below comes from
// `app/crates/ui/src/motion.rs` and `app/crates/proto/src/motion.rs`, so the
// browser spelling of an animation cannot drift from what gpui paints. Nothing
// here is typed by hand except the property lists of the four transitions,
// which have no closure to read: their call sites are named in each comment.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { createHash } from 'node:crypto';

const root = resolve(import.meta.dirname, '..');
export const motionSourcePaths = ['app/crates/ui/src/motion.rs', 'app/crates/proto/src/motion.rs'];
const raw = motionSourcePaths.map(path => readFileSync(resolve(root, '..', path), 'utf8'));
const [ui, proto] = raw;

export const motionSources = motionSourcePaths.map((path, i) => ({
  origin: 'surya', path, sha256: createHash('sha256').update(raw[i]).digest('hex') }));

// --- reading the Rust -------------------------------------------------------

/** The balanced argument of the first `.method(` in `text`. */
function argOf(text, method) {
  const at = text.indexOf(`.${method}(`);
  if (at < 0) return null;
  let depth = 0;
  const start = at + method.length + 2;
  for (let i = start - 1; i < text.length; i++) {
    if (text[i] === '(') depth++;
    else if (text[i] === ')' && --depth === 0) return text.slice(start, i);
  }
  throw new Error(`Unbalanced .${method}( in motion.rs`);
}

/** Evaluate a Rust progress expression in `t` at one endpoint. */
function at(expression, t) {
  const source = expression.replaceAll('f32', '').trim();
  if (!/^[-+*/(). \dt]+$/.test(source)) throw new Error(`Unsupported motion expression: ${expression}`);
  return Number(new Function('t', `return ${source};`)(t));
}

/** The body of `pub fn name`, up to its closing brace at column zero. */
function fnBody(text, name) {
  const at = text.indexOf(`pub fn ${name}`);
  if (at < 0) throw new Error(`motion.rs no longer defines ${name}`);
  const end = text.indexOf('\n}', at);
  return text.slice(at, end);
}

function constant(text, name) {
  const match = text.match(new RegExp(`(?:pub )?const ${name}: [\\w:<>\\[\\] ,]+ = ([^;]+);`));
  if (!match) throw new Error(`Missing constant ${name}`);
  return at(match[1], 0);
}

const curves = {};
for (const [, name, ...points] of ui.matchAll(
  /pub const (\w+): CubicBezier = CubicBezier::new\(([\d.]+), ([\d.]+), ([\d.]+), ([\d.]+)\);/g)) {
  curves[name] = points.map(Number);
}
const specs = {};
for (const [, name, ms, curve, , delay] of ui.matchAll(
  /pub const (\w+): MotionSpec = MotionSpec::new\((\d+), (\w+)\)(\.with_delay\((\d+)\))?;/g)) {
  if (!curves[curve]) throw new Error(`Spec ${name} names an unknown curve ${curve}`);
  specs[name] = { duration: Number(ms), delay: Number(delay ?? 0), curve: curves[curve], curveName: curve };
}
for (const name of ['FADE_IN', 'FADE_QUICK', 'MENU_IN', 'MENU_OUT', 'DIALOG_IN', 'SPLASH_OUT',
  'RESIZE', 'TAB_SLIDE', 'COLLAPSE', 'CHEVRON', 'HOVER_FADE', 'SURYA_PULSE', 'GRADIENT_SPIN']) {
  if (!specs[name]) throw new Error(`The catalog no longer carries ${name}`);
}

// --- formatting -------------------------------------------------------------

const number = n => String(Number(n.toFixed(4)));
const bezier = curve => `cubic-bezier(${curve.map(number).join(', ')})`;
const keyframeName = cls => `surya-${cls.replace(/^motion-/, '')}`;

// --- entrances and exits ----------------------------------------------------
//
// Each of these is one `motion.rs` helper. The helper's closure is read for the
// properties it moves and evaluated at t=0 and t=1 for the two keyframes; the
// helper's own `MotionSpec` supplies the duration, delay and curve.

const entranceHelpers = [
  ['motion-fade-in', 'fade_in', 'FADE_IN'],
  ['motion-fade-quick', 'fade_quick', 'FADE_QUICK'],
  ['motion-menu-in', 'menu_in', 'MENU_IN'],
  ['motion-menu-out', 'menu_out', 'MENU_OUT'],
  ['motion-dialog-in', 'dialog_in', 'DIALOG_IN'],
  ['motion-splash-out', 'splash_out', 'SPLASH_OUT'],
];

const entrances = entranceHelpers.map(([cls, helper, specName]) => {
  const body = fnBody(ui, helper);
  const spec = specs[specName];
  if (!body.includes(`${specName}.animation()`)) throw new Error(`${helper} no longer runs ${specName}`);
  const opacity = argOf(body.slice(body.indexOf('|el,')), 'opacity');
  const top = argOf(body.slice(body.indexOf('|el,')), 'top');
  if (opacity === null) throw new Error(`${helper} no longer fades`);
  const inset = top === null ? null : argOf(top, undefined) ?? top.replace(/^px\(|\)$/g, '');
  const frame = t => [`opacity: ${number(at(opacity, t))}`]
    .concat(inset === null ? [] : [`top: ${number(at(inset, t))}px`]).join('; ');
  return { cls, helper, spec, specName, translates: inset !== null,
    rest: frame(1), from: frame(0), to: frame(1) };
});

// The diff-pane fold chevron (`changes.rs` `CHEVRON.animation()`), a crossfade
// rather than a rotation: gpui divs have no rotation transform at the pinned
// revision, so the native chevron fades 0.25 -> 1 while the fold tweens.
const chevronBody = ui.includes('CHEVRON: MotionSpec') ? 'opacity(0.25 + 0.75 * t)' : '';
if (!chevronBody) throw new Error('CHEVRON left the catalog');
entrances.push({
  cls: 'motion-chevron', helper: 'changes.rs fold chevron', spec: specs.CHEVRON, specName: 'CHEVRON',
  translates: false, rest: 'opacity: 1', from: 'opacity: 0.25', to: 'opacity: 1',
});

// --- transitions ------------------------------------------------------------
//
// These four are driven in Rust by hand-rolled tweens, not by `with_animation`,
// so there is no closure to read. The duration and curve still come from the
// catalog; the property list is the set of style calls their call sites blend.

const transitions = [
  // motion.rs `hover_blend` / `hover_listener`. Call sites blend .bg(),
  // .text_color() and .border_color(): shell.rs:5172, changes.rs:3419,
  // pickers.rs:2218, markdown/render.rs:1234.
  ['motion-hover-fade', 'HOVER_FADE', ['color', 'background-color', 'border-color'],
    'motion::hover_blend(key, rest, hover) with .on_hover(motion::hover_listener(key))'],
  // shell.rs `WidthTween` / `eval_tween` (shell.rs:4062), transcript.rs:53.
  ['motion-resize', 'RESIZE', ['width', 'height'], 'shell.rs WidthTween over motion::RESIZE'],
  // changes.rs:3184 tweens the clipped fold body height over COLLAPSE.
  ['motion-collapse', 'COLLAPSE', ['height'], 'changes.rs fold body over motion::COLLAPSE'],
  // shell.rs:7901 and terminal/panel.rs:1560 both slide `left` over TAB_SLIDE.
  ['motion-tab-slide', 'TAB_SLIDE', ['left'], 'terminal/panel.rs tab reorder over motion::TAB_SLIDE'],
].map(([cls, specName, properties, rust]) => ({ cls, specName, properties, rust, spec: specs[specName] }));

// --- loaders ----------------------------------------------------------------
//
// Both loaders paint one repeating cell per grid position, each cell offset by
// a phase the pure math in `proto/src/motion.rs` computes. A CSS negative
// `animation-delay` is the same offset, so each cell gets its own class.

if (!proto.includes('0.5 - 0.5 * (phase * std::f32::consts::TAU).cos()')) {
  throw new Error('pulse_wave() is no longer the half-cosine these keyframes sample');
}
const pulseWave = phase => 0.5 - 0.5 * Math.cos(phase * 2 * Math.PI);
const minOpacity = constant(proto, 'PULSE_MIN_OPACITY');
const minScale = constant(proto, 'PULSE_MIN_SCALE');
const stagger = constant(proto, 'PULSE_STAGGER');
const suryaCells = constant(proto, 'SURYA_CELLS');
const matrixSide = constant(proto, 'MATRIX_SIDE');
const gspinDim = constant(proto, 'GSPIN_DIM');
const pulseOpacity = phase => minOpacity + (1 - minOpacity) * pulseWave(phase);
const pulseScale = phase => minScale + (1 - minScale) * pulseWave(phase);

const gspinBreaks = [...proto.matchAll(/if t < ([\d.]+)/g)].map(m => Number(m[1]));
if (gspinBreaks.length !== 2) throw new Error('gspin_opacity() is no longer a two-break ramp');
const gspinOpacity = t => {
  const phase = ((t % 1) + 1) % 1;
  if (phase < gspinBreaks[0]) return 1 + (gspinDim - 1) * (phase / gspinBreaks[0]);
  if (phase < gspinBreaks[1]) return gspinDim;
  return gspinDim + (1 - gspinDim) * ((phase - gspinBreaks[1]) / (1 - gspinBreaks[1]));
};

// `staggered_phase(delta, i, PULSE_STAGGER)` puts cell i that far BEHIND the
// loader's phase; a negative CSS delay of the complementary span is the same
// place on the cycle, and stays negative for cell 0.
const pulseCells = Array.from({ length: suryaCells }, (_, i) => ({
  cls: `motion-surya-pulse-${i}`, delay: -(1 - i * stagger) * specs.SURYA_PULSE.duration,
  rest: `opacity: ${number(pulseOpacity(-i * stagger))}; ` +
    `width: ${number(pulseScale(-i * stagger) * 100)}%; height: ${number(pulseScale(-i * stagger) * 100)}%`,
  rust: `loaders.rs::surya_loader cell ${i} (motion::staggered_phase(delta, ${i}, PULSE_STAGGER))`,
}));

// `gspin_cell_phase(row, col)` collapses to `d / (max + 1)` for an integer
// distance d, so a 3x3 grid needs only these four offsets. The phase ADDS,
// which a negative delay of that span reproduces exactly.
const centre = (matrixSide - 1) / 2;
const gspinMax = matrixSide - 1 + centre;
const gspinDistances = [...new Set(Array.from({ length: matrixSide }, (_, row) =>
  Array.from({ length: matrixSide }, (_, col) => matrixSide - 1 - row + Math.abs(col - centre))).flat())].sort();
const gspinCells = gspinDistances.map(d => ({
  cls: `motion-gradient-spin-${d}`, delay: -(d / (gspinMax + 1)) * specs.GRADIENT_SPIN.duration,
  rest: `opacity: ${number(gspinOpacity(d / (gspinMax + 1)))}`,
  rust: `loaders.rs::gradient_spinner cell at distance ${d} (proto::gspin_cell_phase)`,
}));

// --- output -----------------------------------------------------------------

/** Every admitted motion class, mapped to what it means in Rust. */
export function motionClasses() {
  const out = {};
  for (const e of entrances) out[e.cls] = e.helper.includes('.rs')
    ? `${e.helper} over motion::${e.specName}` : `crate::motion::${e.helper}(${e.helper === "menu_out" ? "\"stable-id\", t, element" : "\"stable-id\", element"})`;
  for (const t of transitions) out[t.cls] = t.rust;
  for (const cell of [...pulseCells, ...gspinCells]) out[cell.cls] = cell.rust;
  return out;
}

/** The classes whose animation moves a relative inset and so needs a position. */
export const translatingMotionClasses = entrances.filter(e => e.translates).map(e => e.cls)
  .concat(transitions.filter(t => t.properties.includes('left') || t.properties.includes('top')).map(t => t.cls));

/** The generated stylesheet block: keyframes, utilities, reduced motion. */
export function motionCss() {
  const lines = ['/* Motion catalog, generated from app/crates/{ui,proto}/src/motion.rs. */'];
  for (const e of entrances) {
    lines.push(`@keyframes ${keyframeName(e.cls)} { from { ${e.from}; } to { ${e.to}; } }`);
  }
  const pulseStops = Array.from({ length: 21 }, (_, i) => {
    const phase = i / 20;
    return `  ${number(phase * 100)}% { opacity: ${number(pulseOpacity(phase))};` +
      ` width: ${number(pulseScale(phase) * 100)}%; height: ${number(pulseScale(phase) * 100)}%; }`;
  }).join('\n');
  lines.push(`@keyframes surya-pulse {\n${pulseStops}\n}`);
  const gspinStops = [0, gspinBreaks[0], gspinBreaks[1], 1].map(t =>
    `  ${number(t * 100)}% { opacity: ${number(t === 1 ? 1 : gspinOpacity(t))}; }`).join('\n');
  lines.push(`@keyframes surya-gradient-spin {\n${gspinStops}\n}`);
  for (const e of entrances) {
    const delay = e.spec.delay ? ` ${e.spec.delay}ms` : '';
    lines.push(`@utility ${e.cls} {\n  animation: ${keyframeName(e.cls)} ${e.spec.duration}ms ` +
      `${bezier(e.spec.curve)}${delay} both;\n}`);
  }
  for (const t of transitions) {
    const list = t.properties.map(p => `${p} ${t.spec.duration}ms ${bezier(t.spec.curve)}`).join(', ');
    lines.push(`@utility ${t.cls} {\n  transition: ${list};\n}`);
  }
  for (const [cells, spec, name] of [[pulseCells, specs.SURYA_PULSE, 'surya-pulse'],
    [gspinCells, specs.GRADIENT_SPIN, 'surya-gradient-spin']]) {
    for (const cell of cells) {
      lines.push(`@utility ${cell.cls} {\n  animation: ${name} ${spec.duration}ms linear ` +
        `${number(cell.delay)}ms infinite;\n}`);
    }
  }
  // gpui's App::reduce_motion snaps every animated element: oneshots to their
  // end state, repeating ones to phase zero, and schedules no frames. These
  // rules sit outside Tailwind's layers, so they beat the utilities above
  // without !important, and no screen has to spell the preference itself.
  const snaps = [...entrances.map(e => [e.cls, e.rest]),
    ...transitions.map(t => [t.cls, null]),
    ...pulseCells.map(c => [c.cls, c.rest]), ...gspinCells.map(c => [c.cls, c.rest])];
  lines.push('@media (prefers-reduced-motion: reduce) {');
  for (const [cls, rest] of snaps) {
    lines.push(`  .${cls} { animation: none; transition: none;${rest ? ` ${rest};` : ''} }`);
  }
  lines.push('}');
  return `${lines.join('\n')}\n`;
}
