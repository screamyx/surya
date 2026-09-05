// node --test sandbox/annotate/jsx-stamp.test.mjs
//
// The wrapper runs inside the browser, so it cannot be imported directly.
// These tests stamp a fake dep file with the shape Vite emits, import the
// result, and call it. That exercises the real wrapper text, not a copy.
import test from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { stampJsxDep, isJsxDepRequest, MARKER, JSX_DEP_PATH } from './jsx-stamp.mjs'

const FAKE_DEP = `
function require_jsx_dev_runtime() {
  return {
    jsxDEV: function (type, props, key, isStatic) { return { type, props, fifth: arguments[4] } },
    Fragment: 'frag'
  }
}
${MARKER}
`

const LOC = { fileName: '/abs/path/sandbox/src/screens/InstallGate.tsx', lineNumber: 120, columnNumber: 21 }

async function loadStamped() {
  const file = path.join(os.tmpdir(), `sandbox-stamp-${process.pid}-${Math.random().toString(36).slice(2)}.mjs`)
  fs.writeFileSync(file, stampJsxDep(FAKE_DEP))
  const mod = await import(file)
  fs.unlinkSync(file)
  return mod.default
}

test('a host element carries its source location', async () => {
  const rt = await loadStamped()
  const el = rt.jsxDEV('h1', { className: 'x' }, null, false, LOC, null)
  assert.equal(el.props['data-hl'], 'sandbox/src/screens/InstallGate.tsx:120:21')
  assert.equal(el.props.className, 'x', 'existing props survive')
})

test('a component is not stamped', async () => {
  const rt = await loadStamped()
  const el = rt.jsxDEV(function MyComp() {}, { a: 1 }, null, false, LOC, null)
  assert.equal('data-hl' in el.props, false)
})

test('an element with no location is not stamped', async () => {
  const rt = await loadStamped()
  const el = rt.jsxDEV('div', { b: 2 }, null, false, undefined, null)
  assert.equal('data-hl' in el.props, false)
})

test('React still receives the location argument', async () => {
  const rt = await loadStamped()
  const el = rt.jsxDEV('h1', {}, null, false, LOC, null)
  assert.equal(el.fifth, LOC)
})

test('other exports are untouched', async () => {
  const rt = await loadStamped()
  assert.equal(rt.Fragment, 'frag')
})

test('a missing marker refuses instead of serving an unstamped page', () => {
  assert.throws(() => stampJsxDep('nothing to replace'), /changed shape/)
})

test('the dep request matches with or without the optimiser hash', () => {
  assert.equal(isJsxDepRequest(JSX_DEP_PATH), true)
  assert.equal(isJsxDepRequest(JSX_DEP_PATH + '?v=abc123'), true)
  assert.equal(isJsxDepRequest('/node_modules/.vite/deps/react.js'), false)
})

