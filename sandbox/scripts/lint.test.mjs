import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync, readFileSync } from 'node:fs';
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
    const snippets = [...skill.matchAll(/```(?:tsx|rust)\n([\s\S]*?)\n```/g)].map(m => m[1]);
    assert.equal(snippets.length, 2, `${pair.kind}: exactly one wrong/right pair`);
    for (const [i, file] of [pair.wrong, pair.right].entries()) {
      assert.ok(readFileSync(resolve(root, file), 'utf8').includes(snippets[i]), `${file}: skill excerpt drifted from tested source`);
    }
  }
});
