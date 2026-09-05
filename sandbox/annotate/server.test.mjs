import test from 'node:test';
import http from 'node:http';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { startAnnotator } from './server.mjs';
import { tailnetAdvice } from './tailnet.mjs';
import { lintTree, lintSource } from '../scripts/lint.mjs';
const exec = promisify(execFile);
const pin = { comment: 'More room here', path: '/', query: '?theme=dark&state=seeded',
  sourceLocation: 'sandbox/src/screens/needs_you/rows.tsx:8:5', theme: 'dark', fixture: 'seeded',
  viewport: { w: 1440, h: 900 }, component: 'NeedsYouPane', selector: '[data-row]' };
test('pin lifecycle, durable polling cursor, archive, and invalid-input boundaries', async () => {
  const stateDir = fs.mkdtempSync(path.join(os.tmpdir(), 'sandbox-annotation-test-'));
  const server = await startAnnotator({ listenPort: 0, vitePort: 1, stateDir });
  const base = `http://127.0.0.1:${server.port}/__sandbox/`;
  const post = (endpoint, data) => fetch(base + endpoint, { method: 'POST',
    headers: { 'content-type': 'application/json' }, body: JSON.stringify(data) });
  const cli = (...args) => exec(process.execPath, [path.join(import.meta.dirname, 'run.mjs'), ...args], {
    env: { ...process.env, SANDBOX_ANNOTATE_PORT: String(server.port), SANDBOX_ANNOTATE_STATE: stateDir },
  });
  try {
    const poll = cli('poll', '--timeout', '5');
    const { id } = await (await post('annotate', { ...pin, id: 'forged', status: 'done' })).json();
    const received = JSON.parse((await poll).stdout);
    assert.equal(received.id, id); assert.notEqual(id, 'forged'); assert.equal(received.status, 'sent');
    assert.equal(received.sourceFile, 'sandbox/src/screens/needs_you/rows.tsx');
    assert.equal(received.sourceLine, 8); assert.equal(received.sourceColumn, 5);
    await assert.rejects(cli('poll', '--timeout', '0.1'), e => e.code === 2);
    assert.equal((await post('done', { id, text: '' })).status, 400);
    await cli('reply', id, 'Added room.'); await cli('done', id);
    assert.equal(JSON.parse((await cli('list')).stdout).status, 'done');
    assert.equal(JSON.parse((await cli('list')).stdout).reply, 'Added room.');
    const before = await (await fetch(base + 'annotations')).json();
    await cli('refresh');
    assert.notEqual((await (await fetch(base + 'annotations')).json()).reloadToken, before.reloadToken);
    for (const bad of [null, { ...pin, sourceLocation: null }, { ...pin, sourceLocation: 'sandbox/src/../../secrets.ts:1:1' },
      { ...pin, sourceLocation: 'sandbox/src/screens/needs_you/rows.tsx:99999:1' }, { ...pin, comment: '' }]) {
      assert.equal((await post('annotate', bad)).status, 400);
    }
    assert.equal((await fetch(base + 'annotations', { headers: { origin: 'https://example.com' } })).status, 403);
    const customHost = headers => new Promise(resolve => {
      http.get(base + 'health', { headers }, response => { response.resume(); resolve(response.statusCode); });
    });
    assert.equal(await customHost({ host: 'example.com' }), 403);
    assert.equal(await customHost({ host: 'host.tail123.ts.net:8515', origin: 'https://host.tail123.ts.net:8515' }), 200);
    assert.equal((await fetch(base + 'annotate', { method: 'POST', body: '{}' })).status, 415);
    assert.equal((await fetch(base + 'overlay.css')).status, 200);
    await cli('clear'); assert.equal((await cli('list')).stdout, '');
    assert.ok(fs.readdirSync(stateDir).some(f => /^annotations\..+\.jsonl$/.test(f)));
    await post('annotate', pin);
    assert.equal(JSON.parse((await cli('poll', '--timeout', '1')).stdout).comment, pin.comment);
    await assert.rejects(cli('annotate', 'not an agent communication channel'), e => e.code === 1);
  } finally { server.close(); fs.rmSync(stateDir, { recursive: true, force: true }); }
});
test('tailnet advice uses an existing private mapping or prints an unused-port command', () => {
  const host = 'pc.tail123.ts.net:8515';
  const config = { TCP: { 8515: { HTTPS: true } }, Web: { [host]: { Handlers: { '/': { Proxy: 'http://127.0.0.1:5178' } } } } };
  assert.match(tailnetAdvice(config, 5178), /Owner URL: https:\/\/pc.tail123.ts.net:8515/);
  assert.match(tailnetAdvice(config, 5188), /tailscale serve --bg --https=8516 http:\/\/127.0.0.1:5188/);
  assert.doesNotMatch(tailnetAdvice({ ...config, AllowFunnel: { [host]: true } }, 5178), /Owner URL/);
});
test('annotation tooling CSS is exempt; importing it into a screen is still rejected', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'sandbox-tooling-lint-'));
  try {
    fs.mkdirSync(path.join(dir, 'annotate'));
    fs.writeFileSync(path.join(dir, 'annotate/overlay.css'), '.dock { position: fixed; }');
    assert.deepEqual(lintTree(dir), []);
    assert.ok(lintSource('import "../annotate/overlay.css";').length);
    fs.writeFileSync(path.join(dir, 'annotate/overlay.js'), '\n'.repeat(501));
    assert.match(lintTree(dir).join('\n'), /over 500 lines/);
  } finally { fs.rmSync(dir, { recursive: true, force: true }); }
});
