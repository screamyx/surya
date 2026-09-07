import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdirSync, copyFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
const root = resolve(import.meta.dirname, '..');
const base = process.env.SANDBOX_URL ?? 'http://127.0.0.1:5177';
const out = resolve(root, 'proof');
mkdirSync(out, { recursive: true });
// Per-user, because a fixed /tmp path is another account's file on a shared box.
const audit = process.env.SANDBOX_AUDIT_DIR
  ?? resolve(tmpdir(), `surya-sandbox-audit-${process.getuid?.() ?? 'user'}`);
mkdirSync(audit, { recursive: true });
const browser = await chromium.launch({ headless: true, executablePath: process.env.CHROMIUM_PATH });
const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });
page.setDefaultTimeout(5000);
const failures = [];
page.on('pageerror', error => failures.push(error.message));
const measurements = [];
try {
  for (const theme of ['dark', 'light']) {
    for (const state of ['seeded', 'empty']) {
      await page.goto(`${base}/?proof=1&theme=${theme}&state=${state}`);
      await page.evaluate(() => document.fonts.ready);
      await page.mouse.move(1439, 899);
      assert.equal(await page.locator('[data-row]').count(), state === 'seeded' ? 2 : 0);
      assert.equal(await page.getByRole('heading', { name: 'Needs you' }).count(), 1);
      assert.equal(await page.locator('[aria-current="page"]').innerText(), state === 'seeded' ? 'Needs you\n2' : 'Needs you');
      assert.equal(await page.getByText('Nothing needs you.', { exact: true }).count(), state === 'empty' ? 1 : 0);
      assert.equal(await page.locator('textarea, [contenteditable]').count(), 0);
      await page.screenshot({ path: resolve(out, `${theme}-${state}-1440x900.png`) });
      measurements.push({ theme, state, ...(await page.evaluate(() => {
        const heading = document.querySelector('h1').getBoundingClientRect();
        const row = document.querySelector('[data-row]')?.getBoundingClientRect();
        return { heading: { x: heading.x, y: heading.y, width: heading.width, height: heading.height },
          row: row ? { x: row.x, y: row.y, width: row.width, height: row.height } : null,
          overflow: document.documentElement.scrollWidth > innerWidth };
      })) });
      if (state === 'seeded') {
        const row = page.locator('[data-row]').first();
        const background = () => row.evaluate(el => getComputedStyle(el).backgroundColor);
        const resting = await background();
        await row.hover(); const hover = await background();
        assert.notEqual(hover, resting);
        await page.mouse.down(); assert.notEqual(await background(), resting); await page.mouse.up();
        assert.match(await page.getByRole('status').innerText(), /OpenChat\(wire-tasks\)/);
        assert.equal(await page.locator('[data-row]').count(), 2, 'callback does not optimistically remove rows');
        await page.getByRole('button', { name: 'Dismiss preview event' }).click();
        await row.focus(); await page.keyboard.press('Enter');
        assert.match(await page.getByRole('status').innerText(), /OpenChat/);
      }
    }
  }
  await page.goto(base);
  await page.getByRole('button', { name: 'Use light' }).click();
  assert.equal(await page.locator('[data-theme]').getAttribute('data-theme'), 'light');
  await page.getByRole('button', { name: 'Show empty' }).click();
  assert.equal(await page.locator('[data-row]').count(), 0);
  await page.getByRole('button', { name: 'Show seeded' }).click();
  assert.equal(await page.locator('[data-row]').count(), 2);
  await page.getByRole('button', { name: 'Toggle sidebar', exact: true }).click();
  assert.equal(await page.locator('aside').count(), 0);
  await page.getByRole('button', { name: 'Toggle sidebar', exact: true }).click();
  assert.equal(await page.locator('aside').count(), 1);
  // Every action with no ported destination still emits a visible fixture event.
  for (const label of ['Back', 'Forward', 'New chat', 'Minimize preview', 'Maximize preview',
    'Close preview', 'Wire the Tasks pane into the shell', 'All projects', 'Filter chats']) {
    await page.getByRole('button', { name: label, exact: true }).click();
    assert.match(await page.getByRole('status').innerText(), /Preview event:/);
    await page.getByRole('button', { name: 'Dismiss preview event' }).click();
  }
  // The rail routes, as shell.rs does: the frame stays and the outlet changes.
  // Scope to the shell rail: a ported screen may carry its own aria-current.
  const rail = page.locator('nav[aria-label="Main navigation"] [aria-current="page"]');
  // A rail destination marks itself current; the three off-rail ones clear it.
  // Needs you carries its waiting badge in the accessible name, so match the prefix.
  for (const [label, current] of [['Home', 'Home'], ['Agents', 'Agents'], ['Tasks', 'Tasks'],
    ['Files', 'Files'], ['Needs you', 'Needs you']]) {
    await page.getByRole('button', { name: new RegExp(`^${label}`) }).first().click();
    assert.equal(await page.locator('aside').count(), 1, `${label} keeps the rail`);
    assert.ok((await rail.innerText()).startsWith(current), label);
  }
  for (const label of ['Open browser', 'Toggle right pane', 'Local only']) {
    await page.getByRole('button', { name: label, exact: true }).click();
    assert.equal(await page.locator('aside').count(), 1, `${label} keeps the rail`);
    assert.equal(await rail.count(), 0, label);
  }
  await page.getByRole('button', { name: /^Needs you/ }).first().click();
  // Portable static render for the repository's file-based visual gates.
  for (const theme of ['light', 'dark']) {
    await page.goto(`${base}/?proof=1&theme=${theme}&state=seeded`);
    await page.evaluate(() => document.fonts.ready);
    const css = await page.evaluate(() => [...document.styleSheets].flatMap(sheet => [...sheet.cssRules].map(rule => rule.cssText)).join('\n'));
    const body = await page.locator('body').innerHTML();
    const html = `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Needs you ${theme} audit</title><style>${css.replaceAll('url("/fonts/', `url("file://${root}/public/fonts/`)}</style></head><body>${body.replace(/<script[\s\S]*?<\/script>/g, '')}</body></html>`;
    writeFileSync(resolve(audit, `${theme}-audit.html`), html);
  }
  const references = {
    dark: '/store/surya-gallery/windows-needsyou-seeded-dark-1440x900.png',
    light: '/store/surya-gallery/windows-needsyou-light-1440x900.png',
  };
  for (const [theme, original] of Object.entries(references)) {
    const file = existsSync(original) ? original : resolve(out, `${theme}-windows-reference.png`);
    if (file !== resolve(out, `${theme}-windows-reference.png`)) copyFileSync(file, resolve(out, `${theme}-windows-reference.png`));
    const fixture = theme === 'light' ? 'empty' : 'seeded';
    const data = path => `data:image/png;base64,${readFileSync(path).toString('base64')}`;
    await page.setViewportSize({ width: 2888, height: 900 });
    await page.setContent(`<html><body style="margin:0;display:flex"><img src="${data(file)}"><img src="${data(resolve(out, `${theme}-${fixture}-1440x900.png`))}"></body></html>`);
    await page.locator('img').evaluateAll(images => Promise.all(images.map(image => image.decode())));
    await page.screenshot({ path: resolve(out, `${theme}-comparison.png`) });
  }
  assert.deepEqual(failures, []);
  assert.ok(measurements.every(m => !m.overflow));
  writeFileSync(resolve(out, 'measurements.json'), `${JSON.stringify(measurements, null, 2)}\n`);
  console.log(`Audit HTML written to ${audit}`);
  console.log('Browser proof passed: four theme/state frames, count/badge/empty/no-composer, mouse/keyboard callbacks, fixture switches, sidebar toggle, 9 frame actions, 8 rail destinations, no runtime errors or desktop overflow.');
} finally { await browser.close(); }
