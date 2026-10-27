import assert from 'node:assert/strict'
import { appendFileSync, realpathSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { isAbsolute, join } from 'node:path'

import { dependent, pkg, theme } from './corpus.mjs'
import { capture, encode, hash } from './observation.mjs'
import { cases, code } from './validation-cases.mjs'

const [entry, destination, id] = process.argv.slice(2)
assert.equal(process.argv.length, 5)
assert.ok(isAbsolute(entry) && isAbsolute(destination))
assert.deepEqual(process.execArgv, [])
const require = createRequire(import.meta.url),
  loaded = realpathSync(entry),
  w = require(loaded)
const save = (name, value) =>
  writeFileSync(join(destination, name), JSON.stringify(value) + '\n', {
    flag: 'wx',
  })
save('provenance.json', {
  loaded,
  sha256: hash(loaded),
  runtime: {
    execPath: process.execPath,
    version: process.version,
    versions: process.versions,
    sha256: hash(process.execPath),
  },
  modules: Object.keys(require.cache).map((path) => ({
    path,
    sha256: hash(path),
  })),
})
const test = cases.find((item) => item.id === id)
assert.ok(test, id)
const extract = (filename, text) => {
  const out = w.codeExtract(filename, text, pkg, 'df', false, false, false, {})
  try {
    return { code: out.code, dependencies: out.dependencies }
  } finally {
    out.free()
  }
}
// Given: nonempty observable state; retain a real installed resolver for replacement cases.
w.resetStateForTesting()
w.setPrefix('kept-')
w.setDebug(true)
w.registerTheme(theme)
w.registerShorthands({ insetX: ['left', 'right'] })
w.importCanonicalMap({ 'src/kept.tsx': 'src/root.tsx' })
w.importFileRoutes({ 'src/kept.tsx': [0, 1] })
w.setAtomHoist(2)
const calls = []
w.setModuleResolver((specifier, importer) => {
  calls.push([specifier, importer])
  return {
    path: 'src/validation-token.ts',
    code: 'export const color = "purple";',
  }
})
extract('src/kept.tsx', code)
const snapshot = () => ({
  raw: [
    'exportSheet',
    'exportClassMap',
    'exportFileMap',
    'exportCanonicalMap',
  ].map((name) => capture(() => w[name]())),
  config: [
    'getPrefix',
    'isDebug',
    'getDefaultTheme',
    'getUnreservedClassSlotCount',
  ].map((name) => capture(() => w[name]())),
  interface: capture(() =>
    w.getThemeInterface(
      pkg,
      'Colors',
      'Typography',
      'Length',
      'Shadow',
      'Theme',
    ),
  ),
  css: [null, 0, 1, 2, 3, 4, 7, 128].flatMap((index) =>
    [false, true].map((main) => capture(() => w.getCss(index, main))),
  ),
})
const before = snapshot(),
  args = test.input()
save('before.json', { id, input: encode(args), before })
// When: exactly one invalid position, with every sibling argument valid.
let actual
const result = capture(() => {
  try {
    const value = w[test.fn](...args)
    if (value instanceof w.Output) value.free()
  } catch (error) {
    actual = error
    throw error
  }
})
const trapped = actual instanceof WebAssembly.RuntimeError
const after = trapped ? { tag: 'unobservable-after-wasm-trap' } : snapshot()
save('record.json', {
  id,
  expected: test.expected,
  input: encode(args),
  result,
  before,
  after,
})
// A resolver retention probe is deliberately OUTSIDE the mutation snapshot.
let postprobe, resolvedOutput
if (!trapped && test.fn === 'setModuleResolver') {
  const start = calls.length
  postprobe = capture(() => {
    resolvedOutput = extract('src/fresh-dependent.tsx', dependent)
    return resolvedOutput
  })
  save('postprobe.json', { result: postprobe, calls: calls.slice(start) })
}
// Then: diagnostics exist even when these genuine acceptance assertions fail.
assert.ok(actual instanceof TypeError, `${id}: TypeError required`)
assert.equal(actual.constructor, TypeError)
assert.equal(actual.message, test.expected)
assert.ok(
  [...before.raw, ...before.config, before.interface, ...before.css].every(
    (value) => value.tag === 'return',
  ),
  'snapshot observations must succeed',
)
assert.deepEqual(after, before, `${id}: rejected call mutated state`)
if (test.fn === 'setModuleResolver') {
  assert.equal(postprobe.tag, 'return')
  assert.ok(
    calls.some(
      ([specifier, importer]) =>
        specifier === './tokens' && importer === 'src/fresh-dependent.tsx',
    ),
  )
  assert.deepEqual(resolvedOutput.dependencies, ['src/validation-token.ts'])
}
appendFileSync(
  join(destination, 'complete.json'),
  JSON.stringify({ id }) + '\n',
  { flag: 'wx' },
)
