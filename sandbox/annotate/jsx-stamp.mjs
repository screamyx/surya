// Stamps every React host element with the source location that produced it.
//
// The location is already in the bundle and React throws it away.
// `Vite in development` compiles each element to
// `jsxDEV(type, props, key, isStatic, {fileName, lineNumber, columnNumber}, this)`.
// React 19 declares four parameters, so the fifth argument is passed and then
// discarded. It is still reachable as `arguments[4]`.
//
// So we wrap the dep module's export and copy that location onto the element as
// `data-hl`. The overlay reads it when the user drops a pin. Nothing in the app
// changes, and the wrapper disappears when the proxy stops.
//
// Host elements only. A string `type` is a DOM tag and draws a box the user can
// point at. A function or class `type` is a component and draws nothing itself.

// Vite serves the optimised dep at this path with a `?v=<hash>` query.
// Match the path and ignore the query, because the hash changes.
const JSX_DEP_PATH = '/node_modules/.vite/deps/react_jsx-dev-runtime.js'

// The last line of the dep file. Vite emits exactly this and nothing else in
// the file matches it.
const MARKER = 'export default require_jsx_dev_runtime();'

const WRAPPER = `
const __sandbox_rt = require_jsx_dev_runtime();
const __sandbox_wrap = (fn) => function (type, props, key, isStatic) {
  const src = arguments[4];
  if (src && typeof type === "string" && src.fileName) {
    const rel = String(src.fileName).replace(/^.*?(sandbox\\/src\\/.*)$/, "$1");
    props = Object.assign({}, props, { "data-hl": rel + ":" + src.lineNumber + ":" + src.columnNumber });
  }
  return fn.apply(this, [type, props, key, isStatic, arguments[4], arguments[5]]);
};
const __sandbox_out = Object.assign({}, __sandbox_rt);
for (const k of ["jsxDEV", "jsx", "jsxs"]) {
  if (typeof __sandbox_rt[k] === "function") __sandbox_out[k] = __sandbox_wrap(__sandbox_rt[k]);
}
export default __sandbox_out;
`

export const isJsxDepRequest = (reqUrl) => (reqUrl || '').split('?')[0].endsWith(JSX_DEP_PATH)

// Returns the rewritten body, or throws if the marker has gone.
//
// Throwing is deliberate. A missing marker means every pin on that page loses
// its line and nothing says so. That is worse than a page that refuses to load,
// because the person only finds out when an agent edits the wrong file. This is
// why the proxy fails closed when the dep shape changes.
export function stampJsxDep(body) {
  if (!body.includes(MARKER)) {
    throw new Error(
      `sandbox-live: the React dep module changed shape. Expected to find "${MARKER}". ` +
      'Pins would silently lose their line numbers, so this refuses instead. ' +
      'Fix jsx-stamp.mjs against the new Vite output.'
    )
  }
  return body.replace(MARKER, WRAPPER)
}

export { JSX_DEP_PATH, MARKER }
