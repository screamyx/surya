// Real Vite + Chromium proof. Temporary history never enters the owner's queue.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { createServer } from 'vite';
import { chromium } from 'playwright';
import { startAnnotator } from './server.mjs';
const root = path.resolve(import.meta.dirname, '..');
const output = path.join(root, 'proof');
const stateDir = fs.mkdtempSync(path.join(os.tmpdir(), 'sandbox-annotate-proof-'));
const exec = promisify(execFile);
const vite = await createServer({ root, server: { host: '127.0.0.1', port: 15177, strictPort: false } });
await vite.listen();
const proxy = await startAnnotator({ listenPort: 0, vitePort: vite.httpServer.address().port, stateDir });
const base = `http://127.0.0.1:${proxy.port}`;
const browser = await chromium.launch({ headless: true, ...(process.env.CHROMIUM_PATH ? { executablePath: process.env.CHROMIUM_PATH } : {}) });
const cli = (...args) => exec(process.execPath, [path.join(import.meta.dirname, 'run.mjs'), ...args], {
  env: { ...process.env, SANDBOX_ANNOTATE_PORT: String(proxy.port), SANDBOX_ANNOTATE_STATE: stateDir },
});
const errors = [], proof = [];
const waitFor = async (fn, message) => {
  for (let i = 0; i < 100; i++) {
    if (await fn()) return;
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  throw new Error(message);
};
try {
  for (const [theme, touch] of [['dark', false], ['light', false], ['dark', true]]) {
    const context = await browser.newContext({ viewport: touch ? { width: 390, height: 844 } : { width: 1440, height: 900 },
      hasTouch: touch, isMobile: touch });
    const page = await context.newPage();
    page.on('pageerror', error => errors.push(error.message));
    let connected = false;
    const sockets = [];
    page.on('websocket', socket => sockets.push(socket.url()));
    page.on('console', message => { if (message.text().includes('[vite] connected.')) connected = true; });
    await page.goto(`${base}/?theme=${theme}&state=seeded`);
    await page.locator('[data-row]').first().waitFor();
    await page.locator('#__sandbox-annotate .toggle').waitFor();
    await page.evaluate(() => document.fonts.ready);
    const act = async locator => touch ? locator.tap() : locator.click();
    await act(page.locator('#__sandbox-annotate .toggle'));
    const target = touch ? page.locator('nav button[aria-current="page"]') : page.locator('[data-row]').first();
    const location = await target.getAttribute('data-hl');
    assert.ok(location, 'host element has a source stamp');
    if (touch) await target.tap({ position: { x: 2, y: 2 } });
    else await target.click({ position: { x: 2, y: 2 } });
    await page.locator('#__sandbox-annotate textarea').fill(`Proof only: ${theme} ${touch ? 'touch' : 'mouse'} pin`);
    await act(page.locator('#__sandbox-annotate .add'));
    // Drafts survive a navigation before sending.
    await page.reload();
    await page.locator('#__sandbox-annotate .send').waitFor();
    const pending = cli('poll', '--timeout', '15');
    await act(page.locator('#__sandbox-annotate .send'));
    const pin = JSON.parse((await pending).stdout);
    assert.equal(pin.sourceLocation, location);
    assert.equal(pin.theme, theme); assert.equal(pin.fixture, 'seeded');
    assert.equal(pin.pointer, touch ? 'touch' : 'mouse');
    assert.equal(pin.viewport.w, await page.evaluate(() => innerWidth));
    assert.ok(pin.component.includes('Shell') || pin.component.includes('NeedsYouPane'));
    if (!touch) assert.match(pin.sourceLocation, /^sandbox\/src\/screens\/needs_you\/rows.tsx:\d+:\d+$/);
    assert.equal(await page.getByRole('status').count(), 0, 'picking must not activate the app button');
    await cli('reply', pin.id, 'Verified source location and tap flow; no design edit.');
    await cli('done', pin.id);
    const resolved = page.locator(`#__sandbox-annotate .pin.done[data-pin-id="${pin.id}"]`);
    await resolved.waitFor({ timeout: 10000 });
    await act(resolved);
    assert.ok((await page.locator('#__sandbox-annotate .sheet').innerText()).includes(pin.comment));
    assert.match(await page.locator('#__sandbox-annotate .sheet').innerText(), /Verified source location/);
    await page.screenshot({ path: path.join(output, `annotate-${theme}${touch ? '-touch' : ''}.png`) });
    await act(page.locator('#__sandbox-annotate .close'));
    await waitFor(() => connected, 'Vite HMR connection did not reach the browser');
    assert.ok(sockets.length && sockets.every(url => url.startsWith(`ws://127.0.0.1:${proxy.port}/`)), 'HMR must use the proxy origin');
    if (!touch && theme === 'dark') {
      const file = path.join(root, 'src/screens/needs_you/rows.tsx');
      const original = fs.readFileSync(file, 'utf8');
      try {
        fs.writeFileSync(file, '\n' + original);
        await waitFor(async () => /rows.tsx:8:5$/.test(await page.locator('[data-row]').first().getAttribute('data-hl')), 'HMR lost the new source line');
      } finally { fs.writeFileSync(file, original); }
      await waitFor(async () => /rows.tsx:7:5$/.test(await page.locator('[data-row]').first().getAttribute('data-hl')), 'HMR did not restore the source line');
    }
    const reload = page.waitForEvent('load');
    await cli('refresh'); await reload;
    await page.locator('[data-row]').first().waitFor();
    assert.match(await page.locator('[data-row]').first().getAttribute('data-hl'), /^sandbox\/src\//);
    // Switching fixtures must hide the seeded pins even when the URL stays put.
    await page.getByRole('button', { name: 'Show empty', exact: true }).click();
    await waitFor(async () => (await page.locator('#__sandbox-annotate .pin').count()) === 0, 'pins leaked into the empty fixture');
    proof.push(pin);
    await context.close();
  }
  const direct = await browser.newPage();
  await direct.goto(`http://127.0.0.1:${vite.httpServer.address().port}/`);
  await direct.locator('[data-row]').first().waitFor();
  assert.equal(await direct.locator('[data-hl]').count(), 0);
  assert.equal(await direct.locator('#__sandbox-annotate').count(), 0);
  assert.deepEqual(errors, []);
  fs.writeFileSync(path.join(output, 'annotate-pins.jsonl'), proof.map(pin => JSON.stringify(pin)).join('\n') + '\n');
  console.log(proof.map(pin => JSON.stringify(pin)).join('\n'));
  console.log('Annotation proof passed: mouse light/dark, phone touch, draft reload, poll/reply/done, refresh, HMR, fixture isolation, direct Vite clean.');
} finally {
  await browser.close(); proxy.close(); await vite.close();
  fs.rmSync(stateDir, { recursive: true, force: true });
}
