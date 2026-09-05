// One Vite target; review endpoints adapted from project-jag lavish-live.
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { startViteProxy } from './vite-proxy.mjs';
import { createStore, defaultStateDir } from './state.mjs';
const sandboxRoot = path.resolve(import.meta.dirname, '..');
const prefix = '/__sandbox/';
const assets = { 'overlay.js': 'text/javascript', 'overlay.css': 'text/css' };
export function validatePin(payload) {
  if (!payload || typeof payload !== 'object' || Array.isArray(payload)) throw new Error('Expected a pin');
  if (typeof payload.comment !== 'string' || !payload.comment.trim() || payload.comment.length > 8000) throw new Error('Comment required (8000 characters maximum)');
  const match = /^sandbox\/(src\/(?:[\w-]+\/)*[\w.-]+\.tsx?):([1-9]\d*):([1-9]\d*)$/.exec(payload.sourceLocation);
  if (!match) throw new Error('Pick a stamped sandbox/src element; exact source location required');
  const filename = path.resolve(sandboxRoot, match[1]);
  if (!fs.existsSync(filename) || !fs.realpathSync(filename).startsWith(path.join(sandboxRoot, 'src') + path.sep)) throw new Error('Source file is outside this sandbox');
  if (Number(match[2]) > fs.readFileSync(filename, 'utf8').split('\n').length) throw new Error('Source line is outside the file; reload and pin again');
  if (!['light', 'dark'].includes(payload.theme) || !['seeded', 'empty'].includes(payload.fixture)) throw new Error('Theme and fixture required');
  if (!Number.isFinite(payload.viewport?.w) || !Number.isFinite(payload.viewport?.h)) throw new Error('Viewport required');
  const fields = ['url', 'path', 'query', 'title', 'component', 'selector', 'elementText', 'rect', 'viewport', 'breakpoint', 'pointer', 'theme', 'fixture'];
  return { ...Object.fromEntries(fields.map(k => [k, payload[k]])), comment: payload.comment.trim(),
    surface: 'surya sandbox', framework: 'react', sourceLocation: payload.sourceLocation,
    sourceFile: `sandbox/${match[1]}`, sourceLine: Number(match[2]), sourceColumn: Number(match[3]) };
}
function json(res, status, value) {
  const body = JSON.stringify(value);
  res.writeHead(status, { 'content-type': 'application/json', 'cache-control': 'no-store', 'content-length': Buffer.byteLength(body) });
  res.end(body);
}
async function body(req) {
  const chunks = []; let size = 0;
  for await (const chunk of req) {
    size += chunk.length;
    if (size > 32768) throw new Error('Pin body exceeds 32 KiB');
    chunks.push(chunk);
  }
  return JSON.parse(Buffer.concat(chunks).toString('utf8') || '{}');
}
export async function startAnnotator({ listenPort = 5178, vitePort = 5177, stateDir = defaultStateDir } = {}) {
  const store = createStore(stateDir);
  let reloadToken = Date.now();
  async function api(req, res, pathname) {
    try {
      const asset = pathname.slice(prefix.length);
      if (req.method === 'GET' && Object.hasOwn(assets, asset)) {
        res.writeHead(200, { 'content-type': assets[asset], 'cache-control': 'no-store' });
        return res.end(fs.readFileSync(path.join(import.meta.dirname, asset)));
      }
      if (req.method === 'GET' && asset === 'health') {
        return json(res, 200, { ok: true, pid: process.pid, sandboxRoot, vitePort });
      }
      if (req.method === 'GET' && asset === 'annotations') {
        return json(res, 200, { annotations: store.list(), reloadToken, surface: 'surya sandbox' });
      }
      if (req.method !== 'POST') return json(res, 404, { error: 'Unknown annotation endpoint' });
      if (req.headers['content-type'] !== 'application/json') return json(res, 415, { error: 'JSON required' });
      const payload = await body(req);
      if (asset === 'annotate') return json(res, 200, { ok: true, id: store.add(validatePin(payload)).id });
      if (asset === 'reply' || asset === 'done') {
        if (typeof payload.text !== 'string' || payload.text.length > 8000 || (asset === 'reply' && !payload.text.trim())) throw new Error('Reply text required (8000 characters maximum)');
        store.answer(payload.id, payload.text.trim(), asset === 'done' ? 'done' : 'replied');
      } else if (asset === 'refresh') reloadToken++;
      else if (asset === 'clear') { store.clear(); reloadToken++; }
      else return json(res, 404, { error: 'Unknown annotation endpoint' });
      return json(res, 200, { ok: true });
    } catch (error) { json(res, 400, { error: error.message }); }
  }
  return startViteProxy({ listenPort, vitePort,
    injectHtml: '<script src="/__sandbox/overlay.js" defer></script>',
    handleRequest(req, res) {
      const host = req.headers.host || '';
      const url = new URL(req.url, 'http://localhost');
      const hostname = host.split(':')[0];
      if (!['127.0.0.1', 'localhost'].includes(hostname) && !hostname.endsWith('.ts.net')) {
        json(res, 403, { error: 'Loopback or tailnet host required' }); return true;
      }
      if (req.headers.origin && !['http', 'https'].some(scheme => req.headers.origin === `${scheme}://${host}`)) {
        json(res, 403, { error: 'Same-origin request required' }); return true;
      }
      if (!url.pathname.startsWith(prefix)) return false;
      void api(req, res, url.pathname);
      return true;
    },
  });
}
if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  const port = Number(process.env.SANDBOX_ANNOTATE_PORT || 5178);
  const vitePort = Number(process.env.SANDBOX_VITE_PORT || 5177);
  for (const value of [port, vitePort]) if (!Number.isInteger(value) || value < 1 || value > 65535) throw new Error('Invalid port');
  const server = await startAnnotator({ listenPort: port, vitePort, stateDir: process.env.SANDBOX_ANNOTATE_STATE || defaultStateDir });
  if (!server) process.exitCode = 1;
  else {
    console.log(`surya annotator http://127.0.0.1:${server.port} -> Vite :${vitePort}`);
    for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => { server.close(); process.exit(0); });
  }
}
