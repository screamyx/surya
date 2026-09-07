import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync, readFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { allowedClass, lintSource } from './lint.mjs';
test('source-derived utility pairs and admitted states', () => {
  for (const cls of ['flex', 'w-64', 'px-2.25', 'bg-warning/14', 'focus:bg-element-hover']) assert.ok(allowedClass(cls), cls);
  for (const cls of ['grid', 'grid-cols-2', 'md:flex', 'hover:focus:flex', 'w-[13px]',
    'w-12345', 'bg-red-500', 'text-text/garbage', 'animate-spin', 'transition-all', '!flex',
    'bg-warning/14/5', 'bg-element-hover/14', 'hover:[color:red]']) assert.ok(!allowedClass(cls), cls);
});
test('AST rejects bypasses, hooks, inline paint, and dynamic classes', () => {
  for (const source of [
    'const a = <div className="grid-cols-2" />;',
    'const a = <div style={{width: 13}} />;',
    'const a = <div className={`w-${size}`} />;',
    'const a = <div {...props} />;',
    'import {useEffect as effect} from "react";',
    'import React from "react";',
    'import * as R from "react";',
    'export {useEffect} from "react";',
    'import "./extra.css";',
    'import "../../outside";',
    'const a = <style>{css}</style>;',
    'const a = <svg fill="#ffffff" />;',
    'const a = <div dangerouslySetInnerHTML={{__html: css}} />;',
    'document.body.classList.add("grid");',
    'fetch("/api");',
    'const request = fetch; request("/api");',
  ]) assert.ok(lintSource(source).length > 0, source);
  assert.deepEqual(lintSource('import {useState} from "react"; const a = <div className={on ? "flex bg-surface" : "hidden"} />;'), []);
});
test('actual CLI exits nonzero for a deliberate violation and extra CSS', () => {
  const root = mkdtempSync(resolve(tmpdir(), 'surya-lint-test-'));
  try {
    mkdirSync(resolve(root, 'src'));
    writeFileSync(resolve(root, 'src/Bad.tsx'), '<div className="w-[13px]" />');
    mkdirSync(resolve(root, 'src/dist'));
    writeFileSync(resolve(root, 'src/dist/extra.css'), 'div { color: red; }');
    const result = spawnSync(process.execPath, [resolve(import.meta.dirname, 'lint.mjs'), root], { encoding: 'utf8' });
    assert.equal(result.status, 1);
    assert.match(result.stderr, /Class not in whitelist/);
    assert.match(result.stderr, /extra stylesheet/);
  } finally { rmSync(root, { recursive: true }); }
});

test('three worked wrong/right pairs agree with the example lint CLI', async () => {
  const { examplePairs, lintExample } = await import('./lint-examples.mjs');
  for (const pair of examplePairs) {
    assert.deepEqual(await lintExample(pair.right), [], pair.right);
    const wrong = spawnSync(process.execPath, [resolve(import.meta.dirname, 'lint-examples.mjs'), pair.wrong], { encoding: 'utf8' });
    assert.equal(wrong.status, 1, pair.wrong);
    assert.match(wrong.stderr, /merge the row helper|style is not portable|rewriting the render/);
  }
});
test('right JSX specimens import and execute with the existing helpers', async () => {
  const { createServer } = await import('vite');
  const server = await createServer({ root: resolve(import.meta.dirname, '..'), server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { NeedsYouPane } = await server.ssrLoadModule('/examples/port-in/right/NeedsYou.tsx');
    const { renderRow } = await server.ssrLoadModule('/examples/design/right/rows.tsx');
    const { seededRows } = await server.ssrLoadModule('/src/fixtures.ts');
    let chat;
    const onOpenChat = id => { chat = id; };
    assert.equal(NeedsYouPane({ rows: seededRows, onOpenChat }).type, 'section');
    for (const first of [true, false]) {
      const row = renderRow(seededRows[0], first, onOpenChat);
      assert.ok(row.props.className.split(' ').includes('px-1'));
      row.props.onClick();
      assert.equal(chat, seededRows[0].chatId);
    }
  } finally { await server.close(); }
});

test('skills quote the same wrong/right code that the example lint evaluates', async () => {
  const { examplePairs } = await import('./lint-examples.mjs');
  const root = resolve(import.meta.dirname, '..');
  for (const pair of examplePairs) {
    const skill = readFileSync(resolve(root, `../.claude/skills/sandbox-${pair.kind}/SKILL.md`), 'utf8');
    let snippets = [...skill.matchAll(/```(?:tsx|rust)\n([\s\S]*?)\n```/g)].map(m => m[1]);
    if (pair.kind === 'port-back') {
      const { before } = await import('../examples/port-back/right.mjs');
      assert.equal(snippets.shift(), before.trim());
    }
    assert.equal(snippets.length, 2, `${pair.kind}: exactly one wrong/right pair`);
    for (const [i, file] of [pair.wrong, pair.right].entries()) {
      assert.ok(readFileSync(resolve(root, file), 'utf8').includes(snippets[i]), `${file}: skill excerpt drifted from tested source`);
    }
  }
});

test('the motion catalog is admitted and its near misses are not', () => {
  for (const cls of ['motion-fade-in', 'motion-fade-quick', 'motion-menu-in', 'motion-menu-out',
    'motion-dialog-in', 'motion-splash-out', 'motion-chevron', 'motion-hover-fade', 'motion-resize',
    'motion-collapse', 'motion-tab-slide', 'motion-surya-pulse-0', 'motion-surya-pulse-4',
    'motion-gradient-spin-0', 'motion-gradient-spin-3', 'group', 'group-hover:bg-element-hover',
    'group-active:bg-element-active', 'focus-visible:border-accent', 'focus-within:bg-surface',
  ]) assert.ok(allowedClass(cls), cls);
  for (const cls of [
    // Durations, delays and curves the catalog does not carry.
    'duration-500', 'duration-150', 'delay-150', 'ease-out', 'ease-linear',
    'ease-[cubic-bezier(0.16,1,0.3,1)]', 'transition', 'transition-all', 'transition-colors',
    // Transforms. gpui divs have neither scale nor rotate at the pinned revision.
    'scale-95', 'scale-100', 'rotate-90', 'translate-y-1', 'transform',
    // Stock Tailwind animation, and cells outside the generated grids.
    'animate-spin', 'animate-pulse', 'animate-none', 'motion-fade-out',
    'motion-surya-pulse-5', 'motion-gradient-spin-4', 'motion-scroll-glide',
    // Variants with no gpui pair, and the media-query variants.
    'disabled:opacity-50', 'motion-safe:motion-fade-in', 'motion-reduce:hidden',
    'hover:group-hover:bg-surface', 'group-hover:', 'peer-hover:flex',
  ]) assert.ok(!allowedClass(cls), cls);
});

test('an animation that moves a relative inset must carry a position', () => {
  for (const cls of ['motion-fade-in', 'motion-menu-in', 'motion-menu-out', 'motion-dialog-in',
    'motion-splash-out', 'motion-tab-slide']) {
    assert.match(lintSource(`const a = <div className="${cls}" />;`).join(''), /pair it with relative or absolute/, cls);
    assert.deepEqual(lintSource(`const a = <div className="${cls} relative" />;`), [], cls);
    assert.deepEqual(lintSource(`const a = <div className="${cls} absolute" />;`), [], cls);
  }
  // The catalog helpers that only fade set no inset, so they need no position.
  for (const cls of ['motion-fade-quick', 'motion-chevron', 'motion-hover-fade']) {
    assert.deepEqual(lintSource(`const a = <div className="${cls}" />;`), [], cls);
  }
  // Each branch of a ternary is checked on its own.
  assert.match(lintSource('const a = <div className={on ? "motion-menu-in relative" : "motion-menu-out"} />;').join(''),
    /motion-menu-out moves a relative inset/);
});

test('every admitted motion class paints, at a duration read from motion.rs', async () => {
  const { motionClasses, motionSources } = await import('./motion.mjs');
  const root = resolve(import.meta.dirname, '..');
  const whitelist = JSON.parse(readFileSync(resolve(root, 'whitelist.json'), 'utf8'));
  const css = readFileSync(resolve(root, 'src/tailwind.css'), 'utf8');
  const classes = Object.keys(motionClasses());
  assert.ok(classes.length > 0);
  for (const cls of classes) {
    assert.ok(Object.hasOwn(whitelist.exact, cls), `${cls} missing from the whitelist`);
    assert.ok(css.includes(`@utility ${cls} {`), `${cls} emits no Tailwind utility`);
    assert.match(css, new RegExp(`\\n  \\.${cls} \\{ animation: none; transition: none;`),
      `${cls} does not snap under prefers-reduced-motion`);
  }
  // Provenance: the stylesheet and the whitelist both stamp the Rust they read.
  const provenance = JSON.parse(readFileSync(resolve(root, 'src/theme.ts'), 'utf8')
    .match(/export const provenance = ([\s\S]*?) as const;/)[1]);
  for (const source of motionSources) {
    assert.ok(whitelist.sources.some(s => s.path === source.path && s.sha256 === source.sha256),
      `${source.path} drifted from whitelist.json`);
    assert.ok(provenance.some(s => s.path === source.path && s.sha256 === source.sha256),
      `${source.path} drifted from src/theme.ts`);
  }
  // No duration or delay in the motion block that the catalog cannot produce.
  const block = css.slice(css.indexOf('/* Motion catalog'));
  const catalog = new Set(['0ms']);
  for (const [, ms] of readFileSync(resolve(root, '../app/crates/ui/src/motion.rs'), 'utf8')
    .matchAll(/MotionSpec::new\((\d+),|with_delay\((\d+)\)/g)) catalog.add(`${ms}ms`);
  const cellDelays = new Set([...block.matchAll(/animation: surya-[\w-]+ (\d+)ms linear (-?[\d.]+)ms infinite/g)]
    .map(m => `${m[2]}ms`));
  for (const [, value] of block.matchAll(/(-?[\d.]+ms)/g)) {
    assert.ok(catalog.has(value) || cellDelays.has(value), `${value} is not a catalog timing`);
  }
});

test('a token folded out of the Rust theme stops linting clean', async () => {
  const root = resolve(import.meta.dirname, '..');
  const whitelist = JSON.parse(readFileSync(resolve(root, 'whitelist.json'), 'utf8'));
  const themes = JSON.parse(readFileSync(resolve(root, 'src/theme.ts'), 'utf8')
    .match(/export const themes = ([\s\S]*?) as const;/)[1]);
  const fields = Object.keys(themes.dark).map(key => key.replaceAll('_', '-'));
  // text-dim was folded out of the Rust theme in #183. A class the linter
  // accepts and the stylesheet cannot paint is worse than a rejected one.
  for (const cls of ['text-text-dim', 'bg-text-dim', 'border-text-dim', 'bg-text-dim/50']) {
    assert.ok(!allowedClass(cls), cls);
  }
  for (const family of whitelist.families.filter(f => f.alphaSuffixes)) {
    for (const suffix of family.suffixes) {
      assert.ok(fields.includes(suffix), `${family.prefix}${suffix} names no generated token`);
    }
    // An alpha modifier only agrees with gpui `opacity()` on an opaque token.
    for (const suffix of family.alphaSuffixes) {
      const key = suffix.replaceAll('-', '_');
      for (const mode of ['dark', 'light']) {
        assert.match(themes[mode][key], /\/ 1\)$/, `${family.prefix}${suffix}/50 is not opaque in ${mode}`);
      }
    }
  }
});

test('the mono family and the syntax palette are admitted, and nothing near them', async () => {
  const { faces, families, syntaxPalette } = await import('./native-palettes.mjs');
  const { block, fields } = await import('./rust-colors.mjs');
  const root = resolve(import.meta.dirname, '..');
  const read = path => readFileSync(resolve(root, '..', path), 'utf8').replace(/\/\/[^\n]*/g, '');
  const theme = read('app/crates/ui/src/theme.rs');
  const builtins = read('app/crates/theme/src/builtins.rs');
  const themes = JSON.parse(readFileSync(resolve(root, 'src/theme.ts'), 'utf8')
    .match(/export const themes = ([\s\S]*?) as const;/)[1]);
  const css = readFileSync(resolve(root, 'src/tailwind.css'), 'utf8');

  assert.ok(allowedClass('font-mono'), 'font-mono');
  for (const cls of ['font-serif', 'font-geist', 'font-fixed', 'font-mono-fallback']) {
    assert.ok(!allowedClass(cls), cls);
  }
  // Every face gpui registers must ship and have a rule, or a weight silently
  // synthesizes in the browser and the type reads wrong against the native pane.
  const { sans, mono } = families(theme);
  assert.match(css, new RegExp(`--font-mono: "${mono}", monospace;`));
  for (const [family, constant] of [[sans, 'GEIST'], [mono, 'GEIST_MONO']]) {
    for (const face of faces(read('app/crates/ui/src/typography.rs'), constant)) {
      assert.ok(css.includes(`@font-face { font-family: "${family}"; src: url("/fonts/${face.file}.ttf")`
        + ` format("truetype"); font-weight: ${face.weight}; font-style: ${face.style};`), face.file);
      assert.ok(existsSync(resolve(root, `public/fonts/${face.file}.ttf`)), `${face.file}.ttf not shipped`);
    }
  }

  // Every SyntaxPalette field is a token in both appearances and a text- class.
  const seed = mode => fields(block(block(builtins, `fn comet_${mode}()`), 'variant(Seeds')).syntax;
  for (const mode of ['dark', 'light']) {
    const palette = syntaxPalette(theme, builtins, seed(mode));
    assert.equal(Object.keys(palette).length, 24);
    for (const [field, value] of Object.entries(palette)) {
      const token = `syntax-${field.replaceAll('_', '-')}`;
      assert.ok(allowedClass(`text-${token}`), `text-${token}`);
      // Paint-only in Rust: SyntaxPalette is only ever read through text_color.
      assert.ok(!allowedClass(`bg-${token}`), `bg-${token}`);
      assert.ok(!allowedClass(`border-${token}`), `border-${token}`);
      assert.equal(themes[mode][`syntax_${field}`],
        `rgb(${value.slice(0, 3).join(' ')} / ${Number((value[3] / 255).toFixed(6))})`, `${mode}.${field}`);
      assert.match(css, new RegExp(`--color-${token}: var\\(--surya-${token}\\);`), token);
    }
  }
  // Embedded is a HighlightKind, not a palette field: SyntaxPalette::color()
  // paints it with punctuation, so there is no token of its own to name.
  for (const cls of ['text-syntax-embedded', 'text-syntax-type', 'text-syntax-nonesuch',
    'text-syntax', 'text-comment', 'text-keyword']) assert.ok(!allowedClass(cls), cls);
});

test('the terminal palette is admitted on the verbs that paint it', async () => {
  const { terminalPalette } = await import('./native-palettes.mjs');
  const { block, fields, split, evaluate, color } = await import('./rust-colors.mjs');
  const root = resolve(import.meta.dirname, '..');
  const read = path => readFileSync(resolve(root, '..', path), 'utf8').replace(/\/\/[^\n]*/g, '');
  const theme = read('app/crates/ui/src/theme.rs');
  const builtins = read('app/crates/theme/src/builtins.rs');
  const themes = JSON.parse(readFileSync(resolve(root, 'src/theme.ts'), 'utf8')
    .match(/export const themes = ([\s\S]*?) as const;/)[1]);
  const css = readFileSync(resolve(root, 'src/tailwind.css'), 'utf8');
  const variant = block(builtins, 'fn variant(seed:');

  for (const mode of ['dark', 'light']) {
    const dark = mode === 'dark';
    const seedFields = fields(block(block(builtins, `fn comet_${mode}()`), 'variant(Seeds'));
    const seed = Object.fromEntries(Object.entries(seedFields).filter(([, v]) => /^"#[a-f0-9]{6}"$/i.test(v))
      .map(([k, v]) => [`seed.${k}`, color(JSON.parse(v))]));
    const env = { ...seed, dark };
    for (const statement of split(variant.slice(0, variant.indexOf('ThemeVariant {')), ';')) {
      const m = statement.match(/let (?:mut )?(\w+) = ([\s\S]*)$/);
      if (m && !['accent', 'dark', 'colors'].includes(m[1])) env[m[1]] = evaluate(m[2], env);
    }
    const palette = terminalPalette(theme, builtins, seedFields, env);
    // background, foreground, selection and sixteen ANSI slots.
    assert.equal(Object.keys(palette).length, 19);
    for (const [field, value] of Object.entries(palette)) {
      const token = `terminal-${field.replaceAll('_', '-')}`;
      assert.equal(themes[mode][`terminal_${field}`],
        `rgb(${value.slice(0, 3).join(' ')} / ${Number((value[3] / 255).toFixed(6))})`, `${mode}.${field}`);
      assert.match(css, new RegExp(`--color-${token}: var\\(--surya-${token}\\);`), token);
      // A cell takes its foreground and its background from the same value.
      assert.ok(allowedClass(`bg-${token}`), `bg-${token}`);
      assert.equal(allowedClass(`text-${token}`), token !== 'terminal-selection', `text-${token}`);
      // Nothing in the Rust paints a terminal border.
      assert.ok(!allowedClass(`border-${token}`), `border-${token}`);
    }
  }
  // The selection is a translucent fill, so it takes no alpha modifier either.
  assert.ok(allowedClass('bg-terminal-background/50'), 'bg-terminal-background/50');
  assert.ok(!allowedClass('bg-terminal-selection/50'), 'bg-terminal-selection/50');
  // 16..255 are computed from the index by terminal::view::extended_indexed_rgb,
  // not held by the theme, so there is no token past slot 15.
  for (const cls of ['bg-terminal-ansi-16', 'bg-terminal-ansi-255', 'bg-terminal-ansi-1.0',
    'bg-terminal-ansi', 'bg-terminal', 'bg-ansi-1', 'bg-terminal-cursor']) assert.ok(!allowedClass(cls), cls);
});
