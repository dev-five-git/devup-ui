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
