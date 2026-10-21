import assert from 'node:assert/strict'
import {
  appendFileSync,
  readdirSync,
  realpathSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, isAbsolute, join } from 'node:path'

import { corpus, file, pkg, source } from './corpus.mjs'
import { capture, encode, hash } from './observation.mjs'

export { encode } from './observation.mjs'
const [entry, destination] = process.argv.slice(2)
assert.equal(process.argv.length, 4)
assert.ok(isAbsolute(entry) && isAbsolute(destination))
assert.equal(process.env.NODE_OPTIONS, '')
assert.equal(process.env.BUN_OPTIONS, '')
assert.deepEqual(process.execArgv, [])
const require = createRequire(import.meta.url),
  loaded = realpathSync(entry),
  w = require(loaded)
const save = (name, value) =>
  writeFileSync(
    join(destination, name),
    JSON.stringify(value, null, 2) + '\n',
    { flag: 'wx' },
  )
const modules = Object.keys(require.cache)
const images = [
  ...new Set([
    ...modules,
    ...[...new Set(modules.map(dirname))].flatMap((directory) =>
      readdirSync(directory)
        .filter((name) => name.endsWith('.wasm'))
        .map((name) => join(directory, name)),
    ),
  ]),
]
save('provenance.json', {
  requested: entry,
  loaded,
  runtime: {
    execPath: process.execPath,
    version: process.version,
    versions: process.versions,
    platform: process.platform,
    arch: process.arch,
    execArgv: process.execArgv,
    sha256: hash(process.execPath),
  },
  modules: images.map((path) => ({ path, sha256: hash(path) })),
  scenarios: corpus.map((c) => c.name),
})
const names =
  'Output codeExtract codeExtractWithoutSourceMap exportCanonicalMap exportClassMap exportFileMap exportSheet getCss getDefaultTheme getPrefix getThemeInterface getUnreservedClassSlotCount hasDevupUI importCanonicalMap importClassMap importFileMap importFileRoutes importSheet isDebug registerShorthands registerTheme reserveClassSlots resetBuildState resetStateForTesting seedFileMap setAtomHoist setDebug setModuleResolver setPrefix'.split(
    ' ',
  )
const abi = {
  names: Object.keys(w).sort(),
  types: names.map((name) => [name, typeof w[name]]),
  arities: ['codeExtract', 'codeExtractWithoutSourceMap'].map((name) => [
    name,
    w[name]?.length,
  ]),
}
abi.result = capture(() => {
  assert.deepEqual(abi.names, [...names].sort())
  for (const name of names) assert.equal(typeof w[name], 'function', name)
  for (const [, arity] of abi.arities) assert.equal(arity, 8)
  assert.equal(typeof w.Output.prototype.free, 'function')
})
save('abi.json', abi)
const fields = [
  'code',
  'map',
  'css',
  'cssFile',
  'updatedBaseStyle',
  'dependencies',
]
const exports4 = [
  'exportSheet',
  'exportClassMap',
  'exportFileMap',
  'exportCanonicalMap',
]
let scenario = '',
  sequence = 0
const emit = (record) =>
  appendFileSync(
    join(destination, 'records.jsonl'),
    JSON.stringify({ scenario, sequence: sequence++, ...record }) + '\n',
  )
const exportsRaw = () =>
  Object.fromEntries(exports4.map((name) => [name, w[name]()]))
const snapshot = () => ({
  exports: Object.fromEntries(
    exports4.map((name) => [name, capture(() => w[name]())]),
  ),
  config: Object.fromEntries(
    [
      'getPrefix',
      'isDebug',
      'getDefaultTheme',
      'getUnreservedClassSlotCount',
    ].map((name) => [name, capture(() => w[name]())]),
  ),
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
    [false, true].map((main) => ({
      index,
      main,
      result: capture(() => w.getCss(index, main)),
    })),
  ),
})
const h = {
  w,
  fields,
  exports: exportsRaw,
  note: (label, value) => emit({ kind: 'note', label, value: encode(value) }),
  step(label, action, input) {
    const before = snapshot()
    const result = capture(action)
    emit({
      kind: 'step',
      label,
      input: encode(input),
      before,
      result,
      after: snapshot(),
    })
    return result
  },
  call(name, ...args) {
    return h.step(name, () => w[name](...args), args)
  },
  open(code = source, options = {}) {
    const {
      filename = file,
      method = 'codeExtract',
      single = true,
      flags = [single, false, false],
    } = options
    return w[method](
      filename,
      code,
      pkg,
      'df',
      ...flags,
      Object.hasOwn(options, 'aliases') ? options.aliases : {},
    )
  },
  extract(code = source, options = {}) {
    h.step(
      options.method ?? 'codeExtract',
      () => {
        const out = h.open(code, options)
        try {
          return {
            instanceOfOutput: out instanceof w.Output,
            ...Object.fromEntries(
              fields.map((key) => [
                key,
                key in out ? out[key] : { tag: 'absent' },
              ]),
            ),
          }
        } finally {
          out.free()
        }
      },
      [code, options],
    )
  },
}
assert.equal(corpus.length, 25)
assert.equal(new Set(corpus.map((c) => c.name)).size, 25)
for (const test of corpus) {
  scenario = test.name
  assert.equal(h.call('resetStateForTesting').tag, 'return')
  emit({ kind: 'start', state: snapshot() })
  test.run(h)
  emit({ kind: 'complete', state: snapshot() })
}
if (abi.result.tag !== 'return') process.exitCode = 1
