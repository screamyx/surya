import { readFileSync, writeFileSync, copyFileSync, mkdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { createHash } from 'node:crypto';
import { block, fields, split, color, evaluate, accentRoles } from './rust-colors.mjs';
import { motionCss, motionSourcePaths } from './motion.mjs';
import { faces, families, syntaxPalette, terminalPalette } from './native-palettes.mjs';
const root = resolve(import.meta.dirname, '..');
const files = ['app/crates/ui/src/theme.rs', 'app/crates/theme/src/builtins.rs', 'app/crates/theme/src/lib.rs', 'app/crates/ui/src/icons.rs', 'app/crates/ui/src/typography.rs'];
// The motion catalog is read by scripts/motion.mjs and stamped into the same
// stylesheet, so a curve or duration change lands here through --check too.
const provenanceFiles = [...files, ...motionSourcePaths];
const raw = files.map(path => readFileSync(resolve(root, '..', path), 'utf8'));
const [theme, builtins, lib, icons, typography] = raw.map(s => s.replace(/\/\/[^\n]*/g, ''));
const scalar = [...block(theme, 'pub struct Theme').matchAll(/pub (\w+): Hsla/g)].map(m => m[1]);
const fromVariant = block(theme, 'pub(crate) fn from_variant');
const variant = block(builtins, "fn variant(seed:");
const colorFields = fields(block(variant, 'let colors = ThemeColors'));
const themes = {};
for (const mode of ['light', 'dark']) {
  const dark = mode === 'dark';
  const seedFields = fields(block(block(builtins, `fn comet_${mode}()`), 'variant(Seeds'));
  const seed = Object.fromEntries(Object.entries(seedFields).filter(([, v]) => /^"#[a-f0-9]{6}"$/i.test(v))
    .map(([k, v]) => [`seed.${k}`, color(JSON.parse(v))]));
  const env = { ...seed, dark };
  for (const statement of split(variant.slice(0, variant.indexOf('let colors')), ';')) {
    const m = statement.match(/^let (\w+) = ([\s\S]*)$/);
    if (!m || ['accent', 'dark'].includes(m[1])) continue;
    env[m[1]] = evaluate(m[2], env);
  }
  // `terminal_background` is declared after `let colors`, so sweep the tail too.
  for (const statement of split(variant.slice(variant.indexOf('let colors'), variant.indexOf('ThemeVariant {')), ';')) {
    const m = statement.match(/let (\w+) = ([\s\S]*)$/);
    if (m && m[1] !== 'colors') env[m[1]] = evaluate(m[2], env);
  }
  const accent = accentRoles(lib, seed['seed.accent'], dark, env.background);
  Object.entries(accent).forEach(([key, value]) => { env[`accent.${key}`] = value; });
  const colors = Object.fromEntries(Object.entries(colorFields).map(([key, value]) => [key, evaluate(value, env)]));
  const resolved = {};
  for (const field of scalar) {
    const assignment = fromVariant.match(new RegExp(`theme\\.${field} = model_color\\((colors|accent)\\.(\\w+)\\);`));
    if (assignment) resolved[field] = (assignment[1] === 'colors' ? colors : accent)[assignment[2]];
    else if (field === 'text') resolved[field] = colors.text;
    else if (field === 'text_muted' || field === 'text_dim') resolved[field] = colors.text_muted;
    else if (field === 'on_solid') resolved[field] = colors.on_solid;
    else if (field === 'warning_wash') {
      const helper = block(theme, 'pub fn warning_wash_for');
      const alpha = Number(helper.match(new RegExp(`Appearance::${dark ? 'Dark' : 'Light'} => ([\\d.]+)`))?.[1]);
      if (!Number.isFinite(alpha)) throw new Error('Missing warning wash alpha');
      resolved[field] = [...colors.warning.slice(0, 3), alpha * 255];
    }
    if (!resolved[field]) throw new Error(`No resolved scalar token ${field}`);
  }
  // Follow wash_for(), including its light alpha scale.
  const wash = block(theme, 'fn wash_for(');
  const match = wash.match(new RegExp(`Appearance::${dark ? 'Dark' : 'Light'} => hsla\\(0.0, 0.0, ([\\d.]+), alpha([^)]*)\\)`));
  if (!match) throw new Error('wash_for() changed');
  const scale = match[2].includes('INK_FILL_SCALE') ? Number(theme.match(/const INK_FILL_SCALE: f32 = ([\d.]+)/)[1]) : 1;
  const channel = Number(match[1]) * 255;
  resolved.wash = [channel, channel, channel, scale * 255];
  const brand = block(icons, 'pub fn claude_brand').match(/gpui::rgb\(0x([A-Fa-f0-9]{6})\)/)?.[1];
  if (!brand) throw new Error('Missing native Claude brand color');
  resolved.claude_brand = color(`#${brand}`);
  // Theme.syntax is a SyntaxPalette, not an Hsla, so it never reached the
  // scalar sweep above and every code line in a ported screen painted flat.
  for (const [field, value] of Object.entries(syntaxPalette(theme, builtins, seedFields.syntax))) {
    resolved[`syntax_${field}`] = value;
  }
  // Theme.terminal is a TerminalColors, and its ansi member is an array, so
  // neither reached the sweep either. Nineteen more colours a ported terminal
  // had no way to name.
  for (const [field, value] of Object.entries(terminalPalette(theme, builtins, seedFields, env))) {
    resolved[`terminal_${field}`] = value;
  }
  themes[mode] = Object.fromEntries(Object.entries(resolved).map(([k, c]) => [k,
    `rgb(${c.slice(0, 3).join(' ')} / ${Number((c[3] / 255).toFixed(6))})`]));
}
const { sans: fontSans, mono: fontMono } = families(theme);
// Ship exactly the faces gpui registers, so bold, medium and italic render
// from the same files the native app draws with instead of being synthesized.
const fontFiles = [[fontSans, faces(typography, 'GEIST')], [fontMono, faces(typography, 'GEIST_MONO')]];
const provenanceRaw = provenanceFiles.map(path => readFileSync(resolve(root, '..', path), 'utf8'));
const provenance = provenanceFiles.map((path, i) => ({ path, sha256: createHash('sha256').update(provenanceRaw[i]).digest('hex') }));
const generatedTs = `// Generated by scripts/gen-theme.mjs. Do not edit.\nexport const provenance = ${JSON.stringify(provenance, null, 2)} as const;\nexport const themes = ${JSON.stringify(themes, null, 2)} as const;\nexport type Appearance = keyof typeof themes;\n`;
const scope = mode => Object.entries(themes[mode]).map(([key, value]) => `  --surya-${key.replaceAll('_', '-')}: ${value};`).join('\n');
const themeVars = Object.keys(themes.dark).map(key => `  --color-${key.replaceAll('_', '-')}: var(--surya-${key.replaceAll('_', '-')});`).join('\n');
const { defaultLineHeight } = JSON.parse(readFileSync(resolve(root, 'whitelist.json'), 'utf8'));
const typeVars = [10, 11, 12, 13, 14, 16, 20].map(n => `  --text-ui-${n}: ${n / 16}rem;\n  --text-ui-${n}--line-height: round(nearest, ${defaultLineHeight}em, 1px);`).join('\n');
const fontFaces = fontFiles.flatMap(([family, list]) => list.map(face =>
  `@font-face { font-family: "${family}"; src: url("/fonts/${face.file}.ttf") format("truetype"); font-weight: ${face.weight}; font-style: ${face.style}; font-display: swap; }`)).join('\n');
const css = `/* Generated by scripts/gen-theme.mjs. Do not edit. */\n@import "tailwindcss" source(none);\n@source "./**/*.{ts,tsx}";\n${fontFaces}\n@theme inline {\n  --color-*: initial;\n${themeVars}\n  --font-sans: "${fontSans}", sans-serif;\n  --font-mono: "${fontMono}", monospace;\n${typeVars}\n  --leading-gpui: ${defaultLineHeight};\n}\n:root, [data-theme="dark"] {\n${scope('dark')}\n}\n[data-theme="light"] {\n${scope('light')}\n}\nhtml, body, #root { height: 100%; }\n${motionCss()}`;
for (const [path, contents] of [['src/theme.ts', generatedTs], ['src/tailwind.css', css]]) {
  const target = resolve(root, path);
  if (process.argv.includes('--check')) {
    if (readFileSync(target, 'utf8') !== contents) throw new Error(`${path} is stale. Run npm run generate.`);
  } else writeFileSync(target, contents);
}
if (!process.argv.includes('--check')) {
  mkdirSync(resolve(root, 'public/fonts'), { recursive: true });
  for (const [, list] of fontFiles) for (const face of list) copyFileSync(
    resolve(root, `../app/crates/ui/assets/fonts/${face.file}.ttf`), resolve(root, `public/fonts/${face.file}.ttf`));
  for (const notice of ['Geist-OFL.txt', 'THIRD_PARTY_NOTICES.md']) copyFileSync(
    resolve(root, `../app/crates/ui/assets/fonts/licenses/${notice}`), resolve(root, `public/fonts/${notice}`));
}
console.log(`${process.argv.includes('--check') ? 'Verified' : 'Generated'} ${scalar.length} Theme color fields, ${Object.keys(themes.dark).length - scalar.length - 2} syntax and terminal colors, ${fontFiles.flat(2).length - 2} font faces, light/dark and Tailwind entry`);
