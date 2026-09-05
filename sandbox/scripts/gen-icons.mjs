import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
const root = resolve(import.meta.dirname, '..');
const names = ['sidebar-minimalistic-left', 'sidebar-minimalistic', 'global', 'arrow-left',
  'arrow-right', 'plus', 'close', 'home', 'bell', 'bot', 'checklist', 'folder',
  'alt-arrow-down', 'sort', 'claude-mark', 'git-branch'];
const lines = ['// Generated from app/crates/ui/assets/icons by scripts/gen-icons.mjs.', 'const icons = {'];
for (const name of names) {
  let svg = readFileSync(resolve(root, `../app/crates/ui/assets/icons/${name}.svg`), 'utf8').trim();
  svg = svg.replace(/<svg([^>]*)>/, (_, attrs) => `<svg${attrs.replace(/ (xmlns|width|height)="[^"]*"/g, '')} className="size-full" aria-hidden="true">`);
  svg = svg.replace(/\b(stroke|fill|clip)-([a-z]+)=/g, (_, first, second) => `${first}${second[0].toUpperCase()}${second.slice(1)}=`);
  if (/#[a-f0-9]{3,8}/i.test(svg) || /style=/.test(svg)) throw new Error(`Unexpected paint in ${name}`);
  lines.push(`  '${name}': (${svg}),`);
}
// These two native caption primitives live in shell.rs, not SVG assets.
lines.push('  minus: <svg className="size-full" viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12h14" stroke="currentColor" /></svg>,',
  '  square: <svg className="size-full" viewBox="0 0 24 24" aria-hidden="true"><rect x="6" y="6" width="12" height="12" fill="none" stroke="currentColor" /></svg>,',
  '};', 'export function renderIcon(name: keyof typeof icons) { return icons[name]; }');
const contents = `${lines.join('\n')}\n`;
const target = resolve(root, 'src/shell/icons.tsx');
if (process.argv.includes('--check')) {
  if (readFileSync(target, 'utf8') !== contents) throw new Error('Icons are stale');
} else writeFileSync(target, contents);
