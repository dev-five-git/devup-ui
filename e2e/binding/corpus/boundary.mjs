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
