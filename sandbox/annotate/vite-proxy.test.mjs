// node --test sandbox/annotate/vite-proxy.test.mjs
//
// These run the real proxy against a fake Vite. The fake serves the two things
// that matter - the dep module and an HTML page - plus a WebSocket upgrade, so
// the HMR path is exercised rather than assumed.
//
import test, { after } from 'node:test'
import assert from 'node:assert/strict'
import http from 'node:http'
import net from 'node:net'
import { startViteProxy } from './vite-proxy.mjs'
import { MARKER, JSX_DEP_PATH } from './jsx-stamp.mjs'

const HTML = '<!doctype html><html><head><title>t</title></head><body><h1>hi</h1></body></html>'
const DEP = `function require_jsx_dev_runtime() { return {} }\n${MARKER}\n`

// A stand-in for Vite. `plan` lets one test bend a single response without
// teaching the fake about every case.
function fakeVite(plan = {}) {
  const seen = []
  const server = http.createServer((req, res) => {
    seen.push({ url: req.url, host: req.headers.host, acceptEncoding: req.headers['accept-encoding'] })
    if (req.url.split('?')[0].endsWith(JSX_DEP_PATH)) {
      if (plan.depStatus && plan.depStatus !== 200) {
        res.writeHead(plan.depStatus, { 'content-type': 'application/javascript' })
        return res.end('')
      }
      res.writeHead(200, { 'content-type': 'application/javascript' })
      return res.end(plan.depBody === undefined ? DEP : plan.depBody)
    }
    res.writeHead(200, { 'content-type': 'text/html' })
    res.end(HTML)
  })
  // An upgraded socket leaves the HTTP server's bookkeeping, so closeAllConnections
  // does not reach it and the runner would wait on it forever. Keep them.
  const upgraded = []
  server.on('upgrade', (req, socket) => {
    upgraded.push(socket)
    socket.write('HTTP/1.1 101 Switching Protocols\r\nupgrade: websocket\r\nconnection: Upgrade\r\n\r\n')
    socket.write('hmr-hello')
  })
  return new Promise((resolve) => {
    server.listen(0, '127.0.0.1', () =>
      resolve({
        port: server.address().port,
        seen,
        close: () => { for (const u of upgraded) u.destroy(); server.closeAllConnections(); server.close() },
      })
    )
  })
}

async function withProxy(opts, fn) {
  const vite = await fakeVite(opts.plan)
  const warnings = []
  const proxy = await startViteProxy({
    
    listenPort: 0,
    vitePort: vite.port,
    onWarn: (m) => warnings.push(m),
    injectHtml: opts.injectHtml,
  })
  try {
    return await fn({ proxy, vite, warnings, base: `http://127.0.0.1:${proxy.port}` })
  } finally {
    proxy.close()
    vite.close()
  }
}

// http.request rather than fetch, with keep-alive off. A pooled socket outlives
// the server it points at, and the runner then waits on it after the last
// assertion has already passed.
const get = (base, p, headers = {}) =>
  new Promise((resolve, reject) => {
    const u = new URL(base + p)
    const req = http.request(
      { hostname: u.hostname, port: u.port, path: u.pathname + u.search, headers, agent: false },
      (res) => {
        let body = ''
        res.setEncoding('utf8')
        res.on('data', (c) => (body += c))
        res.on('end', () => resolve({ status: res.statusCode, headers: res.headers, body }))
      }
    )
    req.on('error', reject)
    req.end()
  })

// The proxy talks to Vite through the global agent, which keeps sockets alive.
after(() => http.globalAgent.destroy())

test('the dep module comes back stamped', async () => {
  await withProxy({}, async ({ base }) => {
    const r = await get(base, JSX_DEP_PATH + '?v=abc123')
    assert.equal(r.status, 200)
    assert.match(r.body, /data-hl/)
    // A response carrying both is malformed and every browser rejects it. The
    // fake Vite chunks its answer, which is what surfaced this.
    assert.equal(r.headers['transfer-encoding'], undefined)
    assert.equal(r.headers['content-length'], String(Buffer.byteLength(r.body)))
    assert.doesNotMatch(r.body, new RegExp(MARKER.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')))
  })
})

test('a non-200 dep passes through untouched, because a stale ?v= answers 504', async () => {
  await withProxy({ plan: { depStatus: 504 } }, async ({ base }) => {
    const r = await get(base, JSX_DEP_PATH + '?v=stale')
    assert.equal(r.status, 504)
    assert.equal(r.body, '')
  })
})

test('a dep whose marker has gone refuses with 500 rather than serving a silent miss', async () => {
  await withProxy({ plan: { depBody: 'export default somethingElse();' } },
    async ({ base, warnings }) => {
      const r = await get(base, JSX_DEP_PATH)
      assert.equal(r.status, 500)
      assert.match(r.body, /changed shape/)
      assert.equal(warnings.length, 1)
      assert.match(warnings[0], /sandbox-annotate/)
    })
})

test('HTML is piped untouched when nothing asked for an injection', async () => {
  await withProxy({}, async ({ base }) => {
    const r = await get(base, '/')
    assert.equal(r.body, HTML, 'the transport only injects when asked')
  })
})

test('HTML carries the snippet before </head> when the annotation server asks for one', async () => {
  await withProxy({ injectHtml: '<script>1</script>' }, async ({ base }) => {
    const r = await get(base, '/')
    assert.match(r.body, /<script>1<\/script><\/head>/)
    assert.equal(r.headers['content-length'], String(Buffer.byteLength(r.body)))
  })
})

test('the request reaches Vite as localhost and without accept-encoding', async () => {
  await withProxy({}, async ({ base, vite }) => {
    await get(base, JSX_DEP_PATH, { 'accept-encoding': 'gzip' })
    const hit = vite.seen.at(-1)
    assert.equal(hit.host, `localhost:${vite.port}`, 'Vite checks Host against allowedHosts')
    assert.equal(hit.acceptEncoding, undefined, 'the stamp is a string replace, so the body must be plain')
  })
})

test('the HMR upgrade is forwarded, so a save still updates the page', async () => {
  await withProxy({}, async ({ proxy }) => {
    const said = await new Promise((resolve, reject) => {
      const sock = net.connect(proxy.port, '127.0.0.1', () => {
        sock.write('GET / HTTP/1.1\r\nHost: x\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\r\n')
      })
      let buf = ''
      sock.on('data', (c) => {
        buf += c
        if (buf.includes('hmr-hello')) { sock.destroy(); resolve(buf) }
      })
      sock.on('error', reject)
      setTimeout(() => { sock.destroy(); reject(new Error('no upgrade in 3s')) }, 3000)
    })
    assert.match(said, /101 Switching Protocols/)
    assert.match(said, /hmr-hello/)
  })
})

test('a port already taken returns null rather than throwing', async () => {
  const blocker = http.createServer(() => {})
  await new Promise((r) => blocker.listen(0, '127.0.0.1', r))
  const taken = blocker.address().port
  const warnings = []
  const proxy = await startViteProxy({ listenPort: taken, vitePort: 1, onWarn: (m) => warnings.push(m) })
  blocker.close()
  assert.equal(proxy, null)
  assert.equal(warnings.length, 1)
  assert.match(warnings[0], new RegExp(`sandbox-annotate:${taken}`), 'the warning names the proxy even with no surface')
})

