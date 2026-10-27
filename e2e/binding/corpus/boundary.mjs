import assert from 'node:assert/strict'

import { box, dependent, file, resolved, source } from './extraction.mjs'

const cases = []
const add = (name, run) => cases.push({ name, run })
add('numeric-wrapper-and-boolean-abi', (h) => {
  for (const value of [
    undefined,
    null,
    NaN,
    Infinity,
    -Infinity,
    -1,
    0,
    0.9,
    4294967297,
    '2',
    false,
    true,
    1n,
    Symbol('index'),
  ]) {
    h.call('setAtomHoist', value)
    h.call('getCss', value, false)
    h.call('setDebug', value)
    h.extract(source, { flags: [value, value, value] })
  }
  h.call('setAtomHoist')
  h.call('getCss')
  h.call('setDebug')
})
add('decoder-width-overflow-and-enums', (h) => {
  for (const value of [
    -1,
    1.5,
    NaN,
    Infinity,
    4294967295,
    4294967296,
    Number.MAX_SAFE_INTEGER,
    1n,
  ]) {
    h.call('importFileMap', { [file]: value })
    h.call('importClassMap', { [file]: { color: value } })
    h.call('importFileRoutes', { [file]: [value] })
  }
  h.extract(box('_hover={{color:"red"}} _before={{content:\'"x"\'}}'), {
    single: false,
  })
  const sheet = JSON.parse(h.w.exportSheet())
  h.call('importSheet', sheet)
  for (const value of [
    { css: 4 },
    { css: { broken: [{ selector: { Unknown: [] } }] } },
  ])
    h.call('importSheet', value)
})
add('map-struct-value-getters-undefined-collections', (h) => {
  for (const [name, good, bad] of [
    ['FileMap', { [file]: 4 }, { [file]: NaN }],
    [
      'ClassMap',
      { [file]: new Map([['color', 4]]) },
      { [file]: { color: -1 } },
    ],
    ['CanonicalMap', { [file]: 'src/root.tsx' }, { [file]: 8 }],
    ['Sheet', {}, { css: 4 }],
  ]) {
    h.call(`import${name}`, good)
    h.call(`import${name}`, new Map(Object.entries(good)))
    h.call(`import${name}`, bad)
  }
  const unknown = {
    get unknown() {
      throw new RangeError('unknown getter Ω')
    },
  }
  for (const value of [
    unknown,
    { colors: undefined },
    { colors: null },
    new Map([
      ['colors', new Map([['default', new Map([['primary', 'red']])]])],
    ]),
    { typography: { heading: { fontSize: new Set([12]) } } },
    { typography: { heading: { fontSize: new Map([['nested', 12]]) } } },
  ])
    h.call('registerTheme', value)
  for (const value of [
    unknown,
    { [file]: undefined },
    new Map([[file, 7]]),
    { [file]: 7 },
    Object.create({ inherited: 4 }),
  ])
    h.call('importFileMap', value)
  h.call('importFileRoutes', new Map([[file, new Set([0, 2])]]))
  h.call(
    'registerShorthands',
    new Map([['insetX', new Set(['left', 'right'])]]),
  )
})
add('diagnostic-tojson-cycle-error-getter', (h) => {
  const calls = [],
    cycle = {}
  cycle.self = cycle
  const custom = {
    toJSON() {
      calls.push('toJSON')
      return 'diagnostic Ω'
    },
  }
  for (const value of [
    custom,
    cycle,
    {
      get value() {
        throw new SyntaxError('diagnostic getter Ω')
      },
    },
    () => 1,
    Symbol('value'),
  ])
    h.call('importFileMap', { [file]: value })
  h.note('diagnostic-calls', calls)
})
add('resolver-boundary-order-this-replacement', (h) => {
  const events = []
  const variants = [
    undefined,
    null,
    7,
    () => undefined,
    () => null,
    () => 7,
    () => ({ path: resolved.path, code: 7 }),
    () => {
      throw new TypeError('resolver Ω')
    },
    () => ({
      get path() {
        events.push('path')
        return 7
      },
      get code() {
        events.push('unexpected-code')
        throw new Error('code')
      },
    }),
    () => ({
      get path() {
        events.push('path')
        throw new RangeError('path Ω')
      },
      code: 'x',
    }),
    () => ({
      path: resolved.path,
      get code() {
        events.push('code')
        throw new SyntaxError('code Ω')
      },
    }),
    () => Object.assign(function result() {}, resolved),
    () => resolved,
    () => {
      h.w.setModuleResolver(() => resolved)
      return resolved
    },
  ]
  for (const [i, variant] of variants.entries()) {
    h.call(
      'setModuleResolver',
      typeof variant === 'function'
        ? function (...args) {
            events.push([i, this === null, args])
            return variant()
          }
        : variant,
    )
    h.extract(dependent, { filename: `src/resolver-${i}.tsx` })
    h.extract(dependent, { filename: `src/replaced-${i}.tsx` })
  }
  h.note('resolver-events', events)
  h.call('setModuleResolver', null)
})
add('prefix-valid-and-malformed-boolean-strict', (h) => {
  for (const value of ['du-', null, undefined, '', false, true]) {
    h.call('setPrefix', value)
    h.extract(source)
  }
  h.call('setPrefix')
})
export const boundary = cases

export function capturePatched(patches, action) {
  const originals = patches.map(([target, key]) =>
    Object.getOwnPropertyDescriptor(target, key),
  )
  try {
    for (const [target, key, value] of patches) target[key] = value
    return { value: action() }
  } catch (error) {
    return { error }
  } finally {
    for (let i = 0; i < patches.length; i++) {
      const [target, key] = patches[i]
      const descriptor = originals[i]
      if (descriptor) Object.defineProperty(target, key, descriptor)
      else delete target[key]
    }
  }
}

export function checkGlobalNeighbours(h) {
  for (const [name, target, key] of [
    ['Reflect.get', Reflect, 'get'],
    ['Uint8Array.set', Uint8Array.prototype, 'set'],
  ]) {
    const result = h.step(`global-neighbour-${name}`, () => {
      // Given: a throwing dynamic intrinsic still observed by both real engines.
      const marker = new Error(`observed ${name}`)
      const input = name === 'Reflect.get' ? {} : { probe: new Uint8Array([1]) }
      let calls = 0
      const replacement = () => {
        calls++
        throw marker
      }
      // When: strict map decoding reaches iterator lookup or the typed-byte copy.
      const outcome = capturePatched([[target, key, replacement]], () =>
        h.w.importCanonicalMap(input),
      )
      // Then: iterator-import errors convert once; byte-copy errors stay raw.
      assert.equal(calls, 1)
      if (name === 'Reflect.get') {
        assert.ok(outcome.error instanceof Error)
        assert.equal(outcome.error.name, 'Error')
        assert.notEqual(outcome.error, marker)
        assert.equal(outcome.error.message, `Error: ${marker.message}`)
      } else assert.equal(outcome.error, marker)
    })
    assert.equal(result.tag, 'return', name)
  }
}
