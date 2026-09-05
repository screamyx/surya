// Audits the live page, including alpha-composited and CSS Color 4 backgrounds.
import { chromium } from 'playwright';
import { writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
const browser = await chromium.launch({ headless: true, executablePath: process.env.CHROMIUM_PATH });
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
const report = [];
try {
  for (const theme of ['light', 'dark']) {
    await page.goto(`${process.env.SANDBOX_URL ?? 'http://127.0.0.1:5177'}/?proof=1&theme=${theme}`);
    await page.evaluate(() => document.fonts.ready);
    const controls = await page.locator('button').all();
    for (let i = 0; i < controls.length; i++) {
      const control = controls[i];
      for (const state of ['default', 'hover', 'focus', 'active']) {
        await page.mouse.move(1439, 899);
        await control.evaluate(el => el.blur());
        if (state === 'hover' || state === 'active') await control.hover();
        if (state === 'focus') await control.focus();
        if (state === 'active') await page.mouse.down();
        const samples = await control.evaluate(button => {
          const ctx = document.createElement('canvas').getContext('2d');
          ctx.canvas.width = ctx.canvas.height = 1;
          const rgba = value => {
            ctx.clearRect(0, 0, 1, 1); ctx.fillStyle = value; ctx.fillRect(0, 0, 1, 1);
            return [...ctx.getImageData(0, 0, 1, 1).data];
          };
          const over = (fg, bg) => fg.slice(0, 3).map((v, j) => v * fg[3] / 255 + bg[j] * (1 - fg[3] / 255));
          const background = el => {
            const parents = []; for (let n = el; n; n = n.parentElement) parents.unshift(n);
            return parents.reduce((bg, n) => over(rgba(getComputedStyle(n).backgroundColor), bg), [255, 255, 255]);
          };
          const lum = c => c.map(v => v / 255).map(v => v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4)
            .reduce((sum, v, j) => sum + v * [0.2126, 0.7152, 0.0722][j], 0);
          const ratio = (fg, bg) => (Math.max(lum(fg), lum(bg)) + .05) / (Math.min(lum(fg), lum(bg)) + .05);
          const nodes = [button, ...button.querySelectorAll('*')].filter(el => [...el.childNodes].some(n => n.nodeType === 3 && n.textContent.trim()));
          const graphical = nodes.length === 0;
          return (graphical ? [button.querySelector('svg') ?? button] : nodes).map(el => {
            const style = getComputedStyle(el), bg = background(el), fg = over(rgba(style.color), bg);
            return { label: (el.textContent.trim() || button.getAttribute('aria-label')).slice(0, 70),
              ratio: Number(ratio(fg, bg).toFixed(2)), minimum: graphical ? 3 : 4.5 };
          });
        });
        if (state === 'active') { await page.mouse.move(1439, 899); await page.mouse.up(); }
        report.push(...samples.map(sample => ({ theme, state, ...sample })));
      }
    }
  }
} finally { await browser.close(); }
const failures = report.filter(row => row.ratio < row.minimum);
writeFileSync(resolve(import.meta.dirname, '../proof/contrast.json'), `${JSON.stringify({ checked: report.length, failures }, null, 2)}\n`);
console.log(`Composited contrast: ${report.length - failures.length}/${report.length} text/icon states pass; ${failures.length} native-token limitations recorded in proof/contrast.json.`);
if (failures.length) process.exitCode = 1;
