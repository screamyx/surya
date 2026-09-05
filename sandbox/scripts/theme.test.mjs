import test from 'node:test';
import assert from 'node:assert/strict';
import { color, evaluate, fields, block } from './rust-colors.mjs';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
test('Rust reader respects nested calls and rejects unknown color expressions', () => {
  assert.deepEqual(fields('a: c(seed.a).mix(text, 0.25), b: if dark { card } else { text },'),
    { a: 'c(seed.a).mix(text, 0.25)', b: 'if dark { card } else { text }' });
  assert.deepEqual(evaluate('c(seed.a).with_alpha(if dark { 0.18 } else { 0.10 })',
    { 'seed.a': color('#112233'), dark: true }), [17, 34, 51, 46]);
  assert.throws(() => evaluate('invented_palette()', {}), /Unsupported Rust color expression/);
  assert.throws(() => block('nothing', 'Theme'), /Missing Rust marker/);
});
test('generated theme covers every scalar Theme field in both appearances', () => {
  const root = resolve(import.meta.dirname, '..');
  const source = readFileSync(resolve(root, '../app/crates/ui/src/theme.rs'), 'utf8');
  const scalar = [...block(source, 'pub struct Theme').matchAll(/pub (\w+): Hsla/g)].map(m => m[1]);
  const generated = readFileSync(resolve(root, 'src/theme.ts'), 'utf8');
  const themes = JSON.parse(generated.match(/export const themes = ([\s\S]*?) as const;/)[1]);
  for (const mode of ['dark', 'light']) {
    for (const field of scalar) assert.ok(themes[mode][field], `${mode}.${field}`);
  }
  assert.notEqual(themes.light.surface, themes.dark.surface);
});
