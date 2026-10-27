import assert from 'node:assert/strict'

import { capturePatched as under, checkGlobalNeighbours } from './boundary.mjs'
import {
  box,
  dependent,
  file,
  pkg,
  resolved,
  source,
  theme,
} from './extraction.mjs'

const rows = [
  [file, source, 'df'],
  ['src/second.tsx', box('color="blue"'), 'df'],
]
const cases = []
const add = (name, run) => cases.push({ name, run })
add('reservation-forward-reverse-new-repeat', (h) => {
  for (const replay of [rows, [...rows].reverse()]) {
    h.call('resetBuildState')
    h.call('seedFileMap', ['src/second.tsx', file])
    h.call('reserveClassSlots', rows, pkg, false, false, false, {})
    for (const [filename, code] of replay)
      h.extract(code, { filename, single: false })
    for (let i = 0; i < 2; i++)
      h.extract(box('color="green" gap={gap}'), {
        filename: 'src/new.tsx',
        single: false,
      })
  }
})
add('reservation-malformed-second-row-arity-nonmutation', (h) => {
  h.extract(source)
  for (const value of [
    [rows[0], ['src/bad.tsx', 7, 'df']],
    [[file, source]],
    [[file, source, 'df', 'extra']],
    null,
  ])
    h.call('reserveClassSlots', value, pkg, false, false, false, {})
  h.call('reserveClassSlots', rows, pkg, false, false, false, { bad: 7 })
})
add('four-raw-states-explicit-import-roundtrip', (h) => {
  h.call('registerTheme', theme)
  h.call('importCanonicalMap', { [file]: 'src/root.tsx' })
  h.extract(source, { single: false })
  const packet = h.exports()
  h.call('resetBuildState')
  for (const name of ['Sheet', 'ClassMap', 'FileMap', 'CanonicalMap'])
    h.call(`import${name}`, JSON.parse(packet[`export${name}`]))
  h.call('registerTheme', theme)
  h.extract(source, { single: false })
})
add('distinct-resets-and-resolver-teardown', (h) => {
  for (const reset of ['resetBuildState', 'resetStateForTesting']) {
    const calls = []
    h.call('registerTheme', theme)
    h.call('registerShorthands', { insetX: ['left', 'right'] })
    h.call('setDebug', true)
    h.call('setPrefix', 'dirty-')
    h.call('setAtomHoist', 2)
    h.call('setModuleResolver', (...args) => {
      calls.push(args)
      return resolved
    })
    h.extract(dependent)
    h.call(reset)
    h.extract(dependent)
    h.extract(box('insetX={2} color="$primary"'))
    h.note(`${reset}-calls`, calls)
  }
  const check = (label, action) =>
    assert.equal(h.step(label, action).tag, 'return', label)
  const typeError = (error, message) => {
    assert.ok(error instanceof TypeError)
    assert.equal(error.message, message)
  }
  const open = (aliases) => {
    const out = h.open(source, { aliases })
    try {
      return out instanceof h.w.Output
    } finally {
      out.free()
    }
  }
  for (const mode of ['replaced', 'proxy', 'throwing']) {
    for (const [path, action, message] of [
      ['index', () => open({ '@unused': 'Box' })],
      [
        'slot-zero',
        () => h.w.importCanonicalMap({ probe: 7 }),
        'importCanonicalMap(): argument map_object must be a canonical mapping',
      ],
      [
        'slot-seven',
        () => open({ broken: 7 }),
        'codeExtract(): argument import_aliases must be an alias mapping',
      ],
    ])
      check(`global-Number-${mode}-${path}`, () => {
        // Given: a callable replacement keeps the separately observed safe-int API.
        const original = Number
        const marker = new Error('unexpected global Number call')
        let calls = 0
        const replacement = () => {
          calls++
          if (mode === 'throwing') throw marker
          return 99
        }
        replacement.isSafeInteger = original.isSafeInteger
        // When: index traversal or a schema notification crosses the real boundary.
        const patched =
          mode === 'proxy'
            ? new Proxy(original, { apply: replacement })
            : replacement
        const outcome = under([[globalThis, 'Number', patched]], action)
        // Then: no private key invokes Number; success and canonical slots survive.
        assert.equal(calls, 0)
        if (message) typeError(outcome.error, message)
        else {
          assert.equal(outcome.error, undefined)
          assert.equal(outcome.value, true)
        }
      })
  }
  for (const value of [4294967295n, 4294967296n]) {
    check(`global-String-BigInt-${value}`, () => {
      // Given: primitive BigInts on either side of the unchanged wasm32 bound.
      let calls = 0
      const replacement = () => {
        calls++
        throw new Error('unexpected global String call')
      }
      // When: the real class-slot decoder parses the primitive BigInt.
      const outcome = under([[globalThis, 'String', replacement]], () => {
        h.w.importClassMap({ probe: { slot: value } })
        return h.w.exportClassMap()
      })
      // Then: primitive conversion skips String without changing the bound error.
      assert.equal(calls, 0)
      if (value === 4294967296n)
        typeError(
          outcome.error,
          'importClassMap(): argument sheet_object must be a nested class-index mapping',
        )
      else {
        assert.equal(outcome.error, undefined)
        assert.deepEqual(JSON.parse(outcome.value), {
          probe: { slot: 4294967295 },
        })
      }
    })
  }
  for (const bytes of [new Uint8Array([1]), new ArrayBuffer(1)]) {
    for (const method of ['importCanonicalMap', 'registerTheme']) {
      check(`global-Array.from-${bytes.constructor.name}-${method}`, () => {
        // Given: both typed-byte sources reach borrowed diagnostics or owned buffers.
        let calls = 0
        const replacement = () => {
          calls++
          throw new Error('unexpected Array.from call')
        }
        // When: authored decoding copies bytes before rejecting their domain type.
        const input =
          method === 'registerTheme'
            ? { colors: { default: { probe: bytes } } }
            : { probe: bytes }
        const outcome = under([[Array, 'from', replacement]], () =>
          h.w[method](input),
        )
        // Then: no Array.from transport occurs and the precise public error remains.
        assert.equal(calls, 0)
        typeError(
          outcome.error,
          method === 'registerTheme'
            ? 'registerTheme(): argument theme_object must be a theme'
            : 'importCanonicalMap(): argument map_object must be a canonical mapping',
        )
      })
    }
  }
  for (const method of ['preflight', 'extract']) {
    check(`global-Symbol.iterator-raw-${method}`, () => {
      // Given: instrument both global property reads and the input iterator read.
      const originalSymbol = Symbol,
        originalReflect = Reflect
      const get = originalReflect.get
      const marker = new Error('raw Symbol.iterator property fault'),
        order = []
      const aliases = new Proxy(
        {},
        {
          get(target, key, receiver) {
            order.push('value-iterator')
            return get(target, key, receiver)
          },
        },
      )
      const symbol = new Proxy(originalSymbol, {
        get(target, key, receiver) {
          if (key === 'iterator') {
            order.push('symbol')
            throw marker
          }
          return get(target, key, receiver)
        },
      })
      const reflect = new Proxy(originalReflect, {
        get(target, key, receiver) {
          if (key === 'get') order.push('reflect')
          return get(target, key, receiver)
        },
      })
      // When: either real alias entrypoint looks up an iterator.
      const patches = [
        [globalThis, 'Symbol', symbol],
        [globalThis, 'Reflect', reflect],
      ]
      const outcome = under(patches, () =>
        method === 'preflight'
          ? h.w.hasDevupUI(file, source, pkg, aliases)
          : open(aliases),
      )
      // Then: a Symbol fault stays raw before Reflect.get, never wrapped/forgiven.
      assert.deepEqual(order, ['symbol'])
      assert.equal(outcome.error, marker)
    })
  }
  checkGlobalNeighbours(h)
})
add('output-copies-release-getters-dispose', (h) => {
  h.step('construct-Output', () => new h.w.Output())
  for (const release of ['free', Symbol.dispose]) {
    let out
    h.step('allocate-Output', () => {
      out = h.open(source)
      return out instanceof h.w.Output
    })
    if (!out)
      throw new Error('Output allocation failed outside a boundary-error case')
    h.step('dependency-copy', () => {
      const a = out.dependencies,
        b = out.dependencies
      a.push('local-only')
      return { same: a === b, after: out.dependencies }
    })
    for (const key of h.fields) h.step(`live-${key}`, () => out[key])
    h.step(`release-${String(release)}`, () => out[release]())
    for (const key of [...h.fields, 'free', Symbol.dispose])
      h.step(`released-${String(key)}`, () =>
        typeof key === 'symbol' || key === 'free' ? out[key]() : out[key],
      )
  }
})
export const state = cases
