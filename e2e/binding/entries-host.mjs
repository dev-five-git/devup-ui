import assert from 'node:assert/strict'
import {
  appendFileSync,
  readdirSync,
  realpathSync,
  writeFileSync,
} from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, isAbsolute, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { box, pkg, source } from './corpus/extraction.mjs'
import { runCases } from './entries-cases.mjs'
import { runContext } from './entries-context.mjs'
import { runErrors } from './entries-errors.mjs'
import { encode, hash } from './observation.mjs'

const [entry, destination, host] = process.argv.slice(2)
assert.equal(process.argv.length, 5)
assert.ok(isAbsolute(entry) && isAbsolute(destination))
assert.deepEqual(process.execArgv, [])
assert.equal(process.env.NODE_OPTIONS, '')
assert.equal(process.env.BUN_OPTIONS, '')
assert.equal(typeof process.versions.bun === 'string', host === 'bun')
const require = createRequire(import.meta.url),
  loaded = realpathSync(entry)
const w = require(loaded),
  here = dirname(fileURLToPath(import.meta.url))
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
    sha256: hash(process.execPath),
  },
  modules: images.map((path) => ({ path, sha256: hash(path) })),
  sources: [
    'entries.mjs',
    'entries-host.mjs',
    'entries-cases.mjs',
    'entries-errors.mjs',
    'entries-context.mjs',
    'entries-fixtures.mjs',
    'entries-slots.mjs',
    'entries-queues.mjs',
    'observation.mjs',
    'corpus/extraction.mjs',
  ].map((name) => ({ name, sha256: hash(join(here, name)) })),
})
const take = (action) => {
  try {
    return { caught: false, value: action() }
  } catch (error) {
    return { caught: true, error }
  }
}
const original = Object.getOwnPropertyDescriptor(Object, 'entries')
const patch = (descriptor, action) => {
  Object.defineProperty(Object, 'entries', {
    configurable: true,
    enumerable: original.enumerable,
    ...descriptor,
  })
  try {
    return action()
  } finally {
    Object.defineProperty(Object, 'entries', original)
  }
}
const raw = (action, sentinel) => {
  const result = take(action)
  assert.equal(
    result.caught,
    true,
    'An actual throw is required, including undefined',
  )
  assert.equal(result.error, sentinel, 'Original thrown identity must survive')
}
const schema = (action, message) => {
  const result = take(action)
  assert.equal(result.caught, true)
  assert.ok(result.error instanceof TypeError)
  assert.equal(result.error.constructor, TypeError)
  assert.equal(result.error.message, message)
}
const open = (options = {}) =>
  w[options.mode ?? 'codeExtract'](
    options.file ?? 'src/entries.tsx',
    options.code ?? source,
    pkg,
    'df',
    false,
    false,
    false,
    Object.hasOwn(options, 'aliases') ? options.aliases : {},
  )
const read = (out) => {
  assert.ok(out instanceof w.Output)
  try {
    return [
      'code',
      'map',
      'css',
      'cssFile',
      'updatedBaseStyle',
      'dependencies',
    ].map((key) => [key, out[key]])
  } finally {
    out.free()
  }
}
const snapshot = () => ({
  raw: [
    'exportSheet',
    'exportClassMap',
    'exportFileMap',
    'exportCanonicalMap',
  ].map((name) => w[name]()),
  config: [
    w.getPrefix(),
    w.isDebug(),
    w.getDefaultTheme(),
    w.getUnreservedClassSlotCount(),
  ],
  css: [null, 0, 1].map((index) => w.getCss(index, false)),
})
const ids = []
const h = {
  w,
  take,
  patch,
  raw,
  schema,
  open,
  read,
  snapshot,
  entries(target, result, action) {
    return patch(
      {
        value: function (value) {
          return value === target
            ? result
            : Reflect.apply(original.value, this, [value])
        },
      },
      action,
    )
  },
  fixture(kind) {
    if (kind === 'map')
      return {
        target: {},
        invoke(value) {
          return w.importFileMap(value)
        },
      }
    assert.equal(kind, 'enum')
    read(open({ code: box('_hover={{color:"red"}}') }))
    const packet = JSON.parse(w.exportSheet())
    const find = (value) => {
      if (value === null || typeof value !== 'object') return undefined
      for (const key of Object.keys(value)) {
        const child = value[key]
        if (
          child !== null &&
          typeof child === 'object' &&
          Object.hasOwn(child, 'Selector')
        )
          return { owner: value, key, target: child }
        const result = find(child)
        if (result !== undefined) return result
      }
      return undefined
    }
    const location = find(packet)
    assert.ok(
      location,
      'Actual exported sheet must contain a supported Selector enum',
    )
    const fixture = {
      target: location.target,
      pair: original.value(location.target)[0],
      prepare(action) {
        Object.defineProperty(location.owner, location.key, {
          enumerable: true,
          configurable: true,
          get() {
            action()
            return fixture.target
          },
        })
      },
      invoke() {
        return w.importSheet(packet)
      },
    }
    fixture.prepare(() => {})
    return fixture
  },
  async test(id, action) {
    assert.ok(!ids.includes(id), id)
    w.resetStateForTesting()
    const before = snapshot()
    const observations = await action()
    assert.deepEqual(
      Object.getOwnPropertyDescriptor(Object, 'entries'),
      original,
    )
    appendFileSync(
      join(destination, 'records.jsonl'),
      JSON.stringify({
        id,
        before: encode(before),
        observations: encode(observations),
        after: encode(snapshot()),
      }) + '\n',
    )
    ids.push(id)
  },
}
await runCases(h)
await runErrors(h)
await runContext(h)
save('complete.json', { ids })
