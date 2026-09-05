// Bespoke fork of project-jag lavish-live: one loopback Vite target, HTML injection and JSX stamp.
import http from 'node:http'
import net from 'node:net'
import { isJsxDepRequest, stampJsxDep } from './jsx-stamp.mjs'

// Return the loopback listener, or null on bind failure so startup fails.
export function startViteProxy({ listenPort, vitePort, onWarn, injectHtml, handleRequest }) {
  const warn = onWarn || ((m) => console.warn(m))
  const id = `sandbox-annotate:${listenPort}`
  const sockets = new Set()

  const server = http.createServer((req, res) => {
    if (handleRequest?.(req, res)) return
    const headers = { ...req.headers, host: `localhost:${vitePort}` }
    // The stamp is a string replacement, so the body must arrive uncompressed.
    delete headers['accept-encoding']
    if (isJsxDepRequest(req.url)) { delete headers['if-none-match']; delete headers['if-modified-since'] }

    const upstream = http.request(
      { hostname: '127.0.0.1', port: vitePort, path: req.url, method: req.method, headers },
      (upRes) => {
        const out = { ...upRes.headers }
        // Rewrite a 200 only. Vite answers a stale ?v=<hash> with 504 and an
        // empty body, and a 304 has no body either. Stamping those would turn a
        // condition the browser handles by itself into a hard failure.
        const stampable = isJsxDepRequest(req.url) && upRes.statusCode === 200
        // The annotation server injects its overlay into HTML on this origin.
        const injectable =
          !!injectHtml &&
          upRes.statusCode === 200 &&
          String(upRes.headers['content-type'] || '').includes('text/html')
        if (!stampable && !injectable) {
          res.writeHead(upRes.statusCode, out)
          return upRes.pipe(res)
        }
        const chunks = []
        upRes.on('data', (c) => chunks.push(c))
        upRes.on('end', () => {
          let body
          try {
            const raw = Buffer.concat(chunks).toString('utf8')
            body = stampable ? stampJsxDep(raw) : injectIntoHead(raw, injectHtml)
          } catch (e) {
            // Refuse rather than serve a page whose pins cannot carry a line.
            warn(`[sandbox-annotate] ${id}: ${e.message}`)
            res.writeHead(500, { 'content-type': 'text/plain', 'cache-control': 'no-store' })
            return res.end(String(e.message))
          }
          delete out['content-encoding']
          // Vite normally sends a length, but it is free to chunk. Copying its
          // transfer-encoding and then adding our own length produces a
          // response with both, which undici and every browser reject outright.
          // The rewrite has the whole body in hand, so length is the truth.
          delete out['transfer-encoding']
          out['content-length'] = Buffer.byteLength(body)
          delete out.etag
          out['cache-control'] = 'no-store'
          res.writeHead(upRes.statusCode, out)
          res.end(body)
        })
      }
    )
    upstream.on('error', (e) => {
      res.writeHead(502, { 'content-type': 'text/plain' })
      res.end(`sandbox-annotate: vite :${vitePort} unreachable (${e.code || e.message})`)
    })
    req.pipe(upstream)
  })

  // HMR. Forward the upgrade to Vite and then join the two sockets.
  server.on('upgrade', (req, socket, head) => {
    sockets.add(socket)
    const up = net.connect({ port: vitePort, host: '127.0.0.1' }, () => {
      const lines = [`${req.method} ${req.url} HTTP/1.1`]
      for (const [k, v] of Object.entries({ ...req.headers, host: `localhost:${vitePort}` })) {
        for (const one of Array.isArray(v) ? v : [v]) lines.push(`${k}: ${one}`)
      }
      up.write(lines.join('\r\n') + '\r\n\r\n')
      if (head && head.length) up.write(head)
      up.pipe(socket)
      socket.pipe(up)
    })
    // Either end going away takes the other with it. On 'error' alone, a
    // browser that simply closed the tab left the connection to Vite open, and
    // the sockets accumulated for as long as the proxy ran.
    const drop = () => { sockets.delete(socket); up.destroy(); socket.destroy() }
    up.on('error', drop)
    socket.on('error', drop)
    up.on('close', drop)
    socket.on('close', drop)
  })

  return new Promise((resolve) => {
    server.once('error', (e) => {
      warn(`[sandbox-annotate] ${id}: vite proxy could not bind :${listenPort} (${e.code}). ` +
        'The annotator did not start.')
      resolve(null)
    })
    // Loopback only. Vite binds loopback and this must not widen it: a dev
    // server reads the filesystem, and tailscale serve reaches us over loopback
    // anyway, which is how the off-machine test ran.
    // Report the port the OS actually gave us, not the one we asked for.
    // Identical for a fixed port, and it makes port 0 usable, which is what
    // lets the tests bind without picking numbers that might be in use.
    server.listen(listenPort, '127.0.0.1', () =>
      resolve({
        port: server.address().port,
        close: () => {
          // Drop live sockets too. HMR holds an open WebSocket, and without
          // this the process stays up long after close() was called.
          for (const socket of sockets) socket.destroy()
          server.closeAllConnections()
          server.close()
        },
      })
    )
  })
}

// Put a snippet in the page just before `</head>`, or at the end when there is
// no head.
function injectIntoHead(html, snippet) {
  return /<\/head>/i.test(html) ? html.replace(/<\/head>/i, snippet + '</head>') : html + snippet
}

