// The page as an agent reads it: one line per thing it can act on or read,
// in document order, each actionable one stamped with an id the page keeps
// in `data-surya-id` so a later click or type finds the same element.
//
// Runs through Runtime.evaluate with returnByValue; the result is a string
// so the shape survives any page's own JSON quirks. Kept small on purpose:
// an agent reads this in its context window, so a 5000-node page comes back
// as its first 300 lines and says so.
(() => {
  const MAX_LINES = 300;
  const MAX_TEXT = 120;
  const ACTION = 'a[href],button,input,select,textarea,summary,[role="button"],[role="link"],[role="textbox"],[role="checkbox"],[role="radio"],[role="tab"],[role="menuitem"],[role="option"],[contenteditable="true"],[onclick]';
  const READ = 'h1,h2,h3,h4,h5,h6,p,li,td,th,dt,dd,blockquote,figcaption,label,legend,img[alt]';
  const squash = (s) => (s || '').replace(/\s+/g, ' ').trim();
  const visible = (el) => {
    const r = el.getBoundingClientRect();
    if (r.width <= 0 || r.height <= 0) return false;
    const s = getComputedStyle(el);
    return s.visibility !== 'hidden' && s.display !== 'none' && s.opacity !== '0';
  };
  const roleOf = (el) => {
    const explicit = el.getAttribute('role');
    if (explicit) return explicit;
    const tag = el.tagName.toLowerCase();
    if (tag === 'a') return 'link';
    if (tag === 'button' || tag === 'summary') return 'button';
    if (tag === 'select') return 'combobox';
    if (tag === 'textarea') return 'textbox';
    if (tag === 'input') {
      const t = (el.type || 'text').toLowerCase();
      if (t === 'checkbox' || t === 'radio' || t === 'submit' || t === 'button' || t === 'reset') return t === 'submit' || t === 'reset' ? 'button' : t;
      return 'textbox';
    }
    if (/^h[1-6]$/.test(tag)) return 'heading';
    if (tag === 'img') return 'image';
    if (tag === 'li') return 'listitem';
    if (tag === 'td' || tag === 'th' || tag === 'dt' || tag === 'dd') return 'cell';
    if (el.isContentEditable) return 'textbox';
    return 'text';
  };
  const labelOf = (el) => {
    const own = el.getAttribute('aria-label') || el.getAttribute('title') || el.alt || el.placeholder;
    if (own) return squash(own);
    if (el.labels && el.labels.length) return squash(el.labels[0].innerText);
    if (el.tagName === 'INPUT' && (el.type === 'submit' || el.type === 'button' || el.type === 'reset')) return squash(el.value);
    return squash(el.innerText !== undefined ? el.innerText : el.textContent);
  };
  const lines = [];
  lines.push('url: ' + location.href);
  lines.push('title: ' + squash(document.title));
  lines.push('viewport: ' + innerWidth + 'x' + innerHeight + ' dpr=' + devicePixelRatio + (matchMedia('(pointer: coarse)').matches ? ' touch' : ' mouse'));
  let next = 1;
  let total = 0;
  const actionable = new Set(document.querySelectorAll(ACTION));
  const all = document.querySelectorAll(ACTION + ',' + READ);
  for (const el of all) {
    if (!visible(el)) continue;
    // An element inside a listed control is the control's own text.
    let inside = false;
    for (let p = el.parentElement; p; p = p.parentElement) {
      if (actionable.has(p) && visible(p)) { inside = true; break; }
    }
    if (inside) continue;
    total += 1;
    if (lines.length >= MAX_LINES + 3) continue;
    const role = roleOf(el);
    const label = labelOf(el).slice(0, MAX_TEXT);
    if (actionable.has(el)) {
      const id = next++;
      el.setAttribute('data-surya-id', String(id));
      let extra = '';
      const tag = el.tagName.toLowerCase();
      if (tag === 'a') extra += ' href=' + JSON.stringify(el.getAttribute('href') || '');
      if (tag === 'input' || tag === 'textarea') {
        if (el.type && tag === 'input') extra += ' type=' + el.type;
        if (el.name) extra += ' name=' + JSON.stringify(el.name);
        // A password or a one-time code never leaves the page: the agent
        // sees that the field exists, not what it holds.
        const secret = el.type === 'password' || /one-time-code/.test(el.getAttribute('autocomplete') || '');
        if (el.type === 'checkbox' || el.type === 'radio') extra += el.checked ? ' checked' : '';
        else if (secret) extra += el.value ? ' value=(hidden)' : '';
        else if (el.value) extra += ' value=' + JSON.stringify(String(el.value).slice(0, 80));
      }
      if (tag === 'select') extra += ' value=' + JSON.stringify(el.value);
      if (el.disabled) extra += ' disabled';
      lines.push('[' + id + '] ' + role + ' ' + JSON.stringify(label) + extra);
    } else if (label) {
      lines.push(role + ' ' + JSON.stringify(label));
    }
  }
  const shown = lines.length - 3;
  if (total > shown) lines.push('... ' + (total - shown) + ' more elements not shown');
  return lines.join('\n');
})()
