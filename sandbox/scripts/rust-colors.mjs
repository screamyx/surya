// Deliberately small Rust expression reader. Unsupported syntax fails generation.
export function block(source, marker) {
  const start = source.indexOf(marker);
  if (start < 0) throw new Error(`Missing Rust marker: ${marker}`);
  const open = source.indexOf('{', start + marker.length);
  let depth = 1;
  for (let i = open + 1; i < source.length; i++) {
    if (source[i] === '{') depth++;
    if (source[i] === '}' && --depth === 0) return source.slice(open + 1, i);
    if (source[i] !== '}') continue;
  }
  throw new Error(`Unbalanced Rust block: ${marker}`);
}
export function split(source, separator = ',') {
  const out = [];
  let depth = 0, start = 0, quoted = false;
  for (let i = 0; i < source.length; i++) {
    const c = source[i];
    if (c === '"') quoted = !quoted;
    if (quoted) continue;
    if ('({['.includes(c)) depth++;
    if (')}]'.includes(c)) depth--;
    if (c === separator && depth === 0) { out.push(source.slice(start, i).trim()); start = i + 1; }
  }
  out.push(source.slice(start).trim());
  return out.filter(Boolean);
}
export function fields(source) {
  return Object.fromEntries(split(source).map(part => {
    const m = part.match(/^(\w+):\s*([\s\S]*)$/);
    return m ? [m[1], m[2]] : [part, part];
  }));
}
export const color = hex => hex.slice(1).match(/../g).map(n => parseInt(n, 16)).concat(hex.length === 7 ? [255] : []);
const black = [0, 0, 0, 255], white = [255, 255, 255, 255];
const mix = (a, b, t) => a.map((v, i) => Math.round(v + (b[i] - v) * t));
const lum = c => c.slice(0, 3).map(v => v / 255).map(v => v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4)
  .reduce((s, v, i) => s + v * [0.2126, 0.7152, 0.0722][i], 0);
const contrast = (a, b) => (Math.max(lum(a), lum(b)) + 0.05) / (Math.min(lum(a), lum(b)) + 0.05);
const best = c => contrast(white, c) >= contrast(black, c) ? white : black;
const ensure = (c, bg, min) => {
  if (contrast(c, bg) >= min) return c;
  const target = best(bg);
  for (let step = 1; step <= 20; step++) {
    const candidate = mix(c, target, step / 20);
    if (contrast(candidate, bg) >= min) return candidate;
  }
  return target;
};
export function evaluate(expression, env) {
  const s = expression.trim();
  if (s in env) return env[s];
  if (/^\d+(\.\d+)?$/.test(s)) return Number(s);
  if (/^"#[0-9a-fA-F]{6}"$/.test(s)) return color(JSON.parse(s));
  if (s === 'Color::WHITE') return white;
  if (s === 'Color::BLACK') return black;
  if (s.startsWith('if ')) {
    const m = s.match(/^if (dark|appearance.is_dark\(\))\s*\{([\s\S]*?)\}\s*else\s*\{([\s\S]*)\}$/);
    if (!m) throw new Error(`Unsupported conditional: ${s}`);
    return evaluate(env.dark ? m[2] : m[3], env);
  }
  let m = s.match(/^Color::rgb\((.*)\)$/s);
  if (m) return [...split(m[1]).map(Number), 255];
  m = s.match(/^c\(([\w.]+)\)$/s);
  if (m) return evaluate(m[1], env);
  // Find the last top-level method call; recursion handles chains.
  let depth = 0, dot = -1;
  for (let i = 0; i < s.length; i++) {
    if ('({'.includes(s[i])) depth++;
    if (')}'.includes(s[i])) depth--;
    if (s[i] === '.' && depth === 0 && /[a-z_]/.test(s[i + 1] ?? '')) dot = i;
  }
  if (dot !== -1) {
    m = s.slice(dot + 1).match(/^(\w+)\(([\s\S]*)\)$/);
    if (m) {
      const base = evaluate(s.slice(0, dot), env);
      const args = split(m[2]).map(arg => evaluate(arg, env));
      switch (m[1]) {
        case 'with_alpha': return [...base.slice(0, 3), Math.round(args[0] * 255)];
        case 'mix': return mix(base, args[0], args[1]);
        case 'best_on_color': return best(base);
        case 'ensure_contrast': return ensure(base, args[0], args[1]);
      }
    }
  }
  throw new Error(`Unsupported Rust color expression: ${s}`);
}
export function accentRoles(lib, seed, dark, background) {
  const body = block(lib, 'pub fn derive(primary: Color');
  const env = { primary: seed, dark, background };
  // Evaluate the explicit Rust assignments; no palette values live in JS.
  for (const name of ['primary', 'on', 'strong', 'light', 'deep']) {
    const pattern = new RegExp(`let (?:mut )?${name} = ([\\s\\S]*?);`);
    const expression = body.match(pattern)?.[1];
    if (!expression) throw new Error(`Missing accent assignment ${name}`);
    env[name] = evaluate(expression, env);
  }
  env.strong = ensure(env.strong, env.on, 4.5);
  const roles = fields(block(body, 'Self'));
  return Object.fromEntries(Object.entries(roles).filter(([key]) => key !== 'glyph')
    .map(([key, value]) => [key, evaluate(value, env)]));
}
