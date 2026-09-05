// Tap-based review chrome, forked from project-jag lavish-live.
;(() => {
  if (window.__sandboxAnnotate) return
  window.__sandboxAnnotate = true

  const host = document.createElement('div')
  host.id = '__sandbox-annotate'
  const root = host.attachShadow({ mode: 'open' })
  document.documentElement.appendChild(host)

  const style = document.createElement('link')
  style.rel = 'stylesheet'
  style.href = '/__sandbox/overlay.css'
  root.appendChild(style)

  const el = (tag, cls, html) => {
    const n = document.createElement(tag)
    if (cls) n.className = cls
    if (html) n.innerHTML = html
    return n
  }

  const dock = el('div', 'dock',
    '<span class="mark">surya</span>' +
    '<span class="chip surface">…</span>' +
    '<button class="toggle" type="button" aria-pressed="false">' +
      '<span class="track"><span class="knob"></span></span><span class="tlabel">Annotate</span>' +
    '</button>' +
    '<button class="send" type="button"></button>')
  const hl = el('div', 'hl')
  const scrim = el('div', 'scrim')
  const sheet = el('div', 'sheet')
  const toast = el('div', 'toast')
  root.append(dock, hl, scrim, sheet, toast)

  const toggleBtn = dock.querySelector('.toggle')
  const sendBtn = dock.querySelector('.send')
  const surfaceChip = dock.querySelector('.surface')

  let picking = false
  let pins = []
  let meta = { surface: null, breakpoint: null }

  const DRAFT_KEY = '__sandbox_drafts'
  const loadDrafts = () => { try { return JSON.parse(localStorage.getItem(DRAFT_KEY)) || [] } catch { return [] } }
  const saveDrafts = (d) => { try { localStorage.setItem(DRAFT_KEY, JSON.stringify(d)) } catch {} }
  let drafts = loadDrafts()

  function updateDock() {
    sendBtn.style.display = drafts.length ? 'block' : 'none'
    sendBtn.textContent = `Send ${drafts.length}`
    const bp = breakpointFor(innerWidth)
    surfaceChip.textContent = `${meta.surface || '…'} · ${bp} ${innerWidth}`
  }

  function setPicking(on) {
    picking = on
    toggleBtn.setAttribute('aria-pressed', String(on))
    toggleBtn.querySelector('.tlabel').textContent = on ? 'Pick a thing' : 'Annotate'
    hl.style.display = 'none'
    if (!on) closeSheet()
  }

  toggleBtn.addEventListener('click', (e) => { e.stopPropagation(); setPicking(!picking) })

  // --- picking -----------------------------------------------------------------------------
  // Touch must keep scrolling: a gesture only counts as a pick when the finger
  // barely moved. Blocking touchstart outright would trap the user on one screenful.
  let down = null
  let swallowPickClick = false

  const inOverlay = (e) => e.composedPath().includes(host)

  window.addEventListener('pointerdown', (e) => {
    swallowPickClick = false
    if (!picking || inOverlay(e)) return
    down = { x: e.clientX, y: e.clientY, t: Date.now() }
  }, true)

  window.addEventListener('pointerup', (e) => {
    if (!picking || inOverlay(e) || !down) return
    const moved = Math.hypot(e.clientX - down.x, e.clientY - down.y)
    down = null
    if (moved > 10) return // that was a scroll or a drag, not a pick
    e.preventDefault()
    e.stopImmediatePropagation()
    swallowPickClick = true
    const target = document.elementFromPoint(e.clientX, e.clientY)
    if (target && target !== document.documentElement && target !== document.body) openWriteSheet(target)
  }, true)

  window.addEventListener('pointercancel', () => { down = null }, true)

  // Swallow the click the app would otherwise receive from the same gesture.
  for (const t of ['click', 'mousedown', 'mouseup', 'submit']) {
    window.addEventListener(t, (e) => {
      // A touch click can retarget to the newly opened scrim; swallow it too.
      if (t === 'click' && swallowPickClick) {
        swallowPickClick = false; e.preventDefault(); e.stopImmediatePropagation(); return
      }
      if (!picking || inOverlay(e)) return
      e.preventDefault()
      e.stopImmediatePropagation()
    }, true)
  }

  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && (picking || sheet.style.display === 'flex')) {
      e.stopImmediatePropagation(); closeSheet(); setPicking(false)
    }
  }, true)

  // --- sheet -------------------------------------------------------------------------------
  function openSheet(html) {
    sheet.innerHTML = '<div class="grab"></div>' + html
    sheet.style.display = 'flex'
    scrim.style.display = 'block'
    hl.style.display = 'none'
  }
  function closeSheet() {
    sheet.style.display = 'none'
    scrim.style.display = 'none'
    sheet.innerHTML = ''
  }
  scrim.addEventListener('click', closeSheet)

  function chips(ctx) {
    const bits = [ctx.surface, `${ctx.breakpoint} · ${ctx.viewport.w}px`, ctx.theme]
    return bits.filter(Boolean).map((b) => `<span class="chip">${esc(b)}</span>`).join('')
  }

  function openWriteSheet(node) {
    const ctx = captureContext(node)
    if (!ctx.sourceLocation) { say('No source line here. Pick a stamped screen element.'); return }
    openSheet(
      `<div class="meta">${chips(ctx)}</div>` +
      `<div class="ctx">${ctx.component ? `<span class="name">${esc(ctx.component)}</span> · ` : ''}${esc(ctx.elementText || ctx.selector)}</div>` +
      '<textarea placeholder="What should change here?"></textarea>' +
      '<div class="row"><button class="ghost cancel" type="button">Cancel</button>' +
      '<button class="primary add" type="button">Add pin</button></div>'
    )
    const ta = sheet.querySelector('textarea')
    ta.focus()
    sheet.querySelector('.cancel').addEventListener('click', closeSheet)
    sheet.querySelector('.add').addEventListener('click', () => {
      const comment = ta.value.trim()
      if (!comment) return
      drafts.push({ ...ctx, comment, draftId: 'd' + Date.now().toString(36) })
      saveDrafts(drafts)
      closeSheet()
      updateDock()
      renderPins()
      say(`Queued (${drafts.length}). Keep going, then Send.`)
    })
  }

  function openReadSheet(a, isDraft) {
    const stale = !isDraft && a.viewport && Math.abs(a.viewport.w - innerWidth) > 1
    openSheet(
      `<div class="meta">${chips({
        surface: a.surface, breakpoint: a.breakpoint || breakpointFor(a.viewport?.w || innerWidth),
        viewport: a.viewport || { w: innerWidth }, theme: a.theme,
      })}${stale ? '<span class="chip">pinned at another width</span>' : ''}</div>` +
      `<div class="ctx">${a.component ? `<span class="name">${esc(a.component)}</span> · ` : ''}${esc(a.elementText || a.selector || '')}</div>` +
      `<div class="read"><span class="who">You</span>${esc(a.comment)}</div>` +
      (a.reply ? `<div class="read"><span class="who">Agent</span>${esc(a.reply)}</div>` : '') +
      (isDraft
        ? '<div class="row"><button class="destructive discard" type="button">Discard</button>' +
          '<button class="ghost close" type="button">Close</button></div>'
        : `<div class="meta"><span class="chip">${esc(a.status || 'sent')}</span></div>` +
          '<div class="row"><button class="ghost close" type="button">Close</button></div>')
    )
    sheet.querySelector('.close')?.addEventListener('click', closeSheet)
    sheet.querySelector('.discard')?.addEventListener('click', () => {
      drafts = drafts.filter((x) => x.draftId !== a.draftId)
      saveDrafts(drafts); updateDock(); renderPins(); closeSheet(); say('Draft discarded')
    })
  }

  // The exact source location, if the React stamp reached this page.
  // jsx-stamp.mjs puts `data-hl="file:line:column"` on every host element.
  //
  // Walk up from the pinned node. The user often points at a leaf that carries
  // no stamp of its own, such as an icon inside a stamped button, and the
  // nearest stamped ancestor is the element they meant.
  function sourceLocationOf(node) {
    for (let n = node; n && n !== document.documentElement; n = n.parentElement) {
      const hl = n.getAttribute && n.getAttribute('data-hl')
      if (hl) return hl
    }
    return null
  }

  // --- context capture ---------------------------------------------------------------------
  function captureContext(node) {
    const detected = componentOf(node)
    return {
      url: location.href,
      path: location.pathname,
      query: location.search,
      fixture: document.querySelector('[data-theme] button[aria-label^="Open chat:"]') ? 'seeded' : 'empty',
      title: document.title,
      surface: meta.surface,
      framework: detected.framework,
      component: detected.name,
      componentFile: detected.file,
      sourceLocation: sourceLocationOf(node),
      selector: cssPath(node),
      elementText: (node.innerText || node.value || '').trim().replace(/\s+/g, ' ').slice(0, 140),
      rect: (({ x, y, width, height }) => ({ x, y, width, height }))(node.getBoundingClientRect()),
      viewport: { w: innerWidth, h: innerHeight, dpr: devicePixelRatio || 1 },
      breakpoint: breakpointFor(innerWidth),
      pointer: matchMedia('(pointer: coarse)').matches ? 'touch' : 'mouse',
      theme: document.querySelector('[data-theme]')?.dataset.theme || 'dark',
    }
  }

  // React component names supplement the exact element source stamp.
  function componentOf(node) {
    return reactComponent(node) || { framework: null, name: null, file: null }
  }

  function reactFiber(node) {
    for (let n = node; n; n = n.parentElement) {
      for (const k in n) {
        if (k.startsWith('__reactFiber$') || k.startsWith('__reactInternalInstance$')) return n[k]
      }
    }
    return null
  }

  function reactTypeName(t, depth = 0) {
    if (!t || depth > 3) return null
    if (typeof t === 'string') return null // host element (div, span)
    if (typeof t === 'function') return t.displayName || t.name || null
    if (typeof t === 'object') {
      // memo, forwardRef and lazy wrap the real component one level down.
      return t.displayName || reactTypeName(t.type, depth + 1) || reactTypeName(t.render, depth + 1) || null
    }
    return null
  }

  function reactComponent(node) {
    try {
      const fiber = reactFiber(node)
      if (!fiber) return null
      const names = []
      let file = null
      for (let f = fiber; f && names.length < 3; f = f.return) {
        // React 18 carried _debugSource; React 19 dropped it. Take it when it is
        // there, and let the server resolve the name against the source tree when not.
        if (!file && f._debugSource?.fileName) {
          file = f._debugSource.fileName + (f._debugSource.lineNumber ? ':' + f._debugSource.lineNumber : '')
        }
        const name = reactTypeName(f.type)
        if (name && name !== names[names.length - 1] && !/^(Fragment|Suspense|Profiler)$/.test(name)) names.push(name)
      }
      if (!names.length) return null
      return { framework: 'react', name: names.join(' < '), file }
    } catch { return null }
  }

  function cssPath(node) {
    const parts = []
    let n = node
    while (n && n.nodeType === 1 && parts.length < 5) {
      if (n.id) { parts.unshift('#' + CSS.escape(n.id)); break }
      let part = n.tagName.toLowerCase()
      // Plain class names only - Tailwind arbitrary values ([72px]) break querySelector.
      const cls = [...n.classList].filter((c) => /^[a-zA-Z0-9_-]+$/.test(c) && c.length < 30).slice(0, 2)
      if (cls.length) part += '.' + cls.join('.')
      const parent = n.parentElement
      if (parent) {
        const same = [...parent.children].filter((c) => c.tagName === n.tagName)
        if (same.length > 1) part += `:nth-of-type(${same.indexOf(n) + 1})`
      }
      parts.unshift(part)
      n = parent
    }
    return parts.join(' > ')
  }

  const BREAKPOINTS = [['2xl', 1536], ['xl', 1280], ['lg', 1024], ['md', 768], ['sm', 640]]
  function breakpointFor(w) {
    for (const [name, min] of BREAKPOINTS) if (w >= min) return name
    return 'xs'
  }

  function esc(s) {
    return String(s ?? '').replace(/[&<>"']/g, (c) => (
      { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]
    ))
  }

  // --- send --------------------------------------------------------------------------------
  sendBtn.addEventListener('click', async () => {
    if (sendBtn.disabled) return
    sendBtn.disabled = true
    const batch = [...drafts]
    let sent = 0
    for (const d of batch) {
      try {
        const { draftId, ...payload } = d
        const res = await fetch('/__sandbox/annotate', {
          method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(payload),
        })
        if (!res.ok) throw new Error('rejected')
        drafts = drafts.filter((x) => x.draftId !== draftId)
        sent++
      } catch { break }
    }
    saveDrafts(drafts)
    sendBtn.disabled = false
    updateDock()
    setPicking(false)
    say(drafts.length ? `Sent ${sent}, ${drafts.length} stuck - is the proxy up?` : `Sent ${sent} to the agent`)
    sync()
  })

  // --- pins --------------------------------------------------------------------------------
  function locate(selector) {
    if (!selector) return null
    try { return document.querySelector(selector) } catch {}
    try { return document.querySelector(selector.replace(/([\[\]])/g, '\\$1')) } catch {}
    return null
  }

  function placePin(node, cls, label, onTap, id) {
    const r = node.getBoundingClientRect()
    if (!r || (!r.width && !r.height)) return false
    if (r.bottom < 0 || r.top > innerHeight) return false
    const p = el('button', cls, '')
    p.type = 'button'
    p.dataset.pinId = id
    p.setAttribute('aria-label', `Pin ${label}`)
    p.textContent = label
    Object.assign(p.style, {
      left: Math.min(Math.max(2, r.right - 14), innerWidth - 46) + 'px',
      top: Math.min(Math.max(2, r.top - 14), innerHeight - 46) + 'px',
    })
    p.addEventListener('click', (e) => { e.stopPropagation(); onTap() })
    root.appendChild(p)
    return true
  }

  function sameFrame(a) {
    return a.path === location.pathname && a.query === location.search
      && a.theme === document.querySelector('[data-theme]')?.dataset.theme
      && a.fixture === (document.querySelector('[data-theme] button[aria-label^="Open chat:"]') ? 'seeded' : 'empty')
  }

  function renderPins() {
    root.querySelectorAll('.pin').forEach((p) => p.remove())
    let n = 0
    for (const a of pins) {
      if (!sameFrame(a)) continue
      const node = locate(a.selector)
      if (!node) continue
      const cls = 'pin' + (a.status === 'done' ? ' done' : a.reply ? ' replied' : '') +
        (a.viewport && Math.abs(a.viewport.w - innerWidth) > 1 ? ' stale' : '')
      if (placePin(node, cls, String(n + 1), () => openReadSheet(a, false), a.id)) n++
    }
    for (const d of drafts) {
      if (!sameFrame(d)) continue
      const node = locate(d.selector)
      if (!node) continue
      if (placePin(node, 'pin draft', String(n + 1), () => openReadSheet(d, true), d.draftId)) n++
    }
  }
  addEventListener('scroll', renderPins, true)
  addEventListener('resize', () => { updateDock(); renderPins() })
  setInterval(renderPins, 800) // SPA route changes swap the DOM with no cheap hook

  function say(msg) {
    toast.textContent = msg
    toast.style.display = 'block'
    clearTimeout(say._t)
    say._t = setTimeout(() => { toast.style.display = 'none' }, 2600)
  }

  // --- sync --------------------------------------------------------------------------------
  let lastReloadToken = null
  async function sync() {
    try {
      const res = await fetch('/__sandbox/annotations?path=' + encodeURIComponent(location.pathname))
      const data = await res.json()
      if (data.surface) meta.surface = data.surface
      if (lastReloadToken !== null && data.reloadToken !== lastReloadToken) {
        say('Agent shipped a fix - reloading')
        setTimeout(() => location.reload(), 600)
      }
      lastReloadToken = data.reloadToken
      const before = JSON.stringify(pins.map((p) => [p.id, p.status, p.reply]))
      pins = data.annotations || []
      updateDock()
      if (JSON.stringify(pins.map((p) => [p.id, p.status, p.reply])) !== before) renderPins()
    } catch {}
  }
  sync()
  setInterval(sync, 4000)
  updateDock()
  renderPins()
})()
