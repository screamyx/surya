#!/usr/bin/env node
// Annotation lifecycle and pin loop, adapted from lavish-live.
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { defaultStateDir } from './state.mjs';
import { printTailnet } from './tailnet.mjs';
const directory = process.env.SANDBOX_ANNOTATE_STATE || defaultStateDir;
const port = Number(process.env.SANDBOX_ANNOTATE_PORT || 5178);
const vitePort = Number(process.env.SANDBOX_VITE_PORT || 5177);
const origin = `http://127.0.0.1:${port}`;
const sandboxRoot = path.resolve(import.meta.dirname, '..');
const cursorFile = path.join(directory, 'poll.cursor');
const pidFile = path.join(directory, 'server.pid');
fs.mkdirSync(directory, { recursive: true });
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
async function request(endpoint, data) {
  const response = await fetch(`${origin}/__sandbox/${endpoint}`, {
    signal: AbortSignal.timeout(3000), ...(data === undefined ? {} : {
      method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(data),
    }),
  });
  const result = await response.json();
  if (!response.ok) throw new Error(result.error || `HTTP ${response.status}`);
  return result;
}
async function health() {
  try { const h = await request('health'); return h.sandboxRoot === sandboxRoot && h.vitePort === vitePort ? h : null; }
  catch { return null; }
}
const [, , command, ...args] = process.argv;
try {
  switch (command) {
    case 'start': {
      if (!await health()) {
        const vite = await fetch(`http://127.0.0.1:${vitePort}/@vite/client`, { signal: AbortSignal.timeout(3000) });
        if (!vite.ok || !(await vite.text()).includes('WebSocket')) throw new Error(`Start sandbox Vite on :${vitePort} first`);
        const log = fs.openSync(path.join(directory, 'server.log'), 'a');
        const child = spawn(process.execPath, [path.join(import.meta.dirname, 'server.mjs')], {
          detached: true, stdio: ['ignore', log, log], env: process.env,
        });
        child.unref(); fs.closeSync(log);
        let ready;
        for (let i = 0; i < 30; i++) {
          ready = await health();
          if (ready?.pid === child.pid || child.exitCode !== null) break;
          await sleep(200);
        }
        if (ready?.pid !== child.pid) throw new Error('Annotator failed to start; see annotate/.state/server.log');
        fs.writeFileSync(pidFile, String(child.pid));
      }
      console.log(`surya annotator: ${origin}/?theme=dark&state=seeded (Vite :${vitePort})`);
      printTailnet(port);
      break;
    }
    case 'status': {
      const h = await health();
      console.log(h ? `Running pid ${h.pid}: ${origin} -> Vite :${vitePort}` : 'Annotator is not running for this checkout');
      printTailnet(port);
      break;
    }
    case 'stop': {
      const h = await health();
      const pid = fs.existsSync(pidFile) ? Number(fs.readFileSync(pidFile, 'utf8')) : null;
      if (!h || h.pid !== pid) throw new Error('No matching annotator process owned by this checkout');
      process.kill(pid, 'SIGTERM'); fs.rmSync(pidFile, { force: true }); console.log('Stopped');
      break;
    }
    case 'poll': {
      const timeout = args.length ? (args[0] === '--timeout' && args.length === 2 ? Number(args[1]) : NaN) : 570;
      if (!Number.isFinite(timeout) || timeout <= 0) throw new Error('Usage: poll [--timeout seconds]');
      const cursor = fs.existsSync(cursorFile) ? fs.readFileSync(cursorFile, 'utf8') : '';
      const deadline = Date.now() + timeout * 1000;
      for (;;) {
        const { annotations } = await request('annotations');
        const batch = annotations.slice(annotations.findIndex(a => a.id === cursor) + 1);
        if (batch.length) {
          for (const pin of batch) console.log(JSON.stringify(pin));
          fs.writeFileSync(cursorFile, batch.at(-1).id);
          break;
        }
        if (Date.now() >= deadline) { console.error('TIMEOUT: no new pins; re-arm poll'); process.exitCode = 2; break; }
        await sleep(500);
      }
      break;
    }
    case 'list':
      for (const pin of (await request('annotations')).annotations) console.log(JSON.stringify(pin));
      break;
    case 'reply':
    case 'done':
      await request(command, { id: args[0], text: args.slice(1).join(' ') });
      console.log(command === 'reply' ? 'Replied' : 'Marked done');
      break;
    case 'refresh':
    case 'clear':
      await request(command, {});
      console.log(command === 'clear' ? 'Cleared; history archived in annotate/.state/' : 'Open pages will reload within four seconds');
      break;
    default:
      console.log('Usage: node annotate/run.mjs start|stop|status|poll [--timeout seconds]|list|reply <id> <text>|done <id> [text]|refresh|clear');
      if (command) process.exitCode = 1;
  }
} catch (error) { console.error(error.message); process.exitCode = 1; }
