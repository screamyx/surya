import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { allowedClass, lintSource } from './lint.mjs';
const root = resolve(import.meta.dirname, '..');
const whitelist = JSON.parse(readFileSync(resolve(root, 'whitelist.json'), 'utf8'));
export const portInContract = {
  component: 'NeedsYouPane',
  delegate: { name: 'renderRow', module: 'src/screens/needs_you/rows', ownedAttribute: 'data-row' },
};
export const examplePairs = [
  { kind: 'port-in', wrong: 'examples/port-in/wrong/NeedsYou.tsx', right: 'examples/port-in/right/NeedsYou.tsx' },
  { kind: 'design', wrong: 'examples/design/wrong/rows.tsx', right: 'examples/design/right/rows.tsx' },
  { kind: 'port-back', wrong: 'examples/port-back/wrong.mjs', right: 'examples/port-back/right.mjs' },
];
// A source-checked minimal-patch contract, not a general Rust parser/compiler.
// The caller supplies the native unit record: px at the 16px reference root.
export function lintPaddingPatch(patch) {
  const errors = [];
  const family = whitelist.families.find(f => f.prefix === 'px-');
  if (!family?.gpui.startsWith('.px(') || !allowedClass(patch.fromClass)
    || !allowedClass(patch.toClass) || !/^px-\d+(\.\d+)?$/.test(patch.fromClass)
    || !/^px-\d+(\.\d+)?$/.test(patch.toClass)) return ['Padding change has no admitted .px(...) mapping'];
  if (patch.target !== 'app/crates/ui/src/inbox/chrome.rs') return ['Patch must target the existing row_line helper'];
  const source = readFileSync(resolve(root, '..', patch.target), 'utf8');
  if (!source.includes(patch.before)) errors.push('Native before fragment no longer matches the source');
  if (patch.unit !== 'px' || patch.rootFontPx !== 16) return [...errors, 'Record the native px unit and 16px reference root'];
  const call = cls => `.px(px(${(Number(cls.slice(3)) / 4 * patch.rootFontPx).toFixed(1)}))`;
  const previous = call(patch.fromClass), next = call(patch.toClass);
  if (patch.before.split(previous).length !== 2) errors.push('Expected one original padding call');
  if (patch.after !== patch.before.replace(previous, next)) {
    errors.push('Change only the mapped padding call; rewriting the render loses native behavior');
  }
  return errors;
}
export async function lintExample(file) {
  const pair = examplePairs.find(p => p.wrong === file || p.right === file);
  if (!pair) throw new Error(`Unknown worked example: ${file}`);
  if (pair.kind === 'port-back') return lintPaddingPatch(await import(pathToFileURL(resolve(root, file)).href));
  return lintSource(readFileSync(resolve(root, file), 'utf8'), file,
    pair.kind === 'port-in' ? portInContract : undefined);
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const file = process.argv[2];
  if (file) {
    const errors = await lintExample(file);
    if (errors.length) { console.error(errors.join('\n')); process.exitCode = 1; }
    else console.log(`${file}: example lint passed`);
  } else {
    for (const pair of examplePairs) {
      const right = await lintExample(pair.right), wrong = await lintExample(pair.wrong);
      if (right.length || !wrong.length) throw new Error(`${pair.kind}: example and lint disagree: ${right.join('; ')}`);
    }
    console.log('Worked-example lint passed: 3 right accepted, 3 wrong rejected');
  }
}
