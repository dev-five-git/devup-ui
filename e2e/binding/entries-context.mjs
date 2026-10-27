import assert from 'node:assert/strict'

import { pkg } from './corpus/extraction.mjs'
import { reserve } from './entries-fixtures.mjs'
import {
  runAliasConversions,
  runQueues,
  runResolverOrder,
} from './entries-queues.mjs'

export async function runContext(h) {
  const { w } = h
  const barrel = "import {Box} from './barrel'; export const v=<Box p={2}/>"
  const resolved = {
    path: 'src/barrel.ts',
    code: `export {Box} from '${pkg}';`,
  }
  for (const phase of ['enumeration', 'length']) {
    for (const install of [true, false])
      await h.test(`resolver-before-core-${phase}-${install}`, () => {
        const events = [],
          aliases = {}
        const resolver = function (specifier, importer) {
          assert.equal(this, null)
          events.push([specifier, importer])
          return resolved
        }
        w.setModuleResolver(install ? null : resolver)
        const change = () => {
          events.push('change')
          w.setModuleResolver(install ? resolver : null)
        }
        let action
        if (phase === 'enumeration') {
          Object.defineProperty(aliases, 'unused', {
            enumerable: true,
            get() {
              change()
              return null
            },
          })
          action = () => w.hasDevupUI('src/before.tsx', barrel, pkg, aliases)
        } else {
          action = () =>
            h.entries(
              aliases,
              {
                get length() {
                  change()
                  return 0
                },
              },
              () => w.hasDevupUI('src/before.tsx', barrel, pkg, aliases),
            )
        }
        assert.equal(action(), install)
        assert.equal(events[0], 'change')
        assert.equal(events.some(Array.isArray), install)
        return events
      })
  }
  for (const method of [
    'codeExtract',
    'codeExtractWithoutSourceMap',
    'reserveClassSlots',
  ]) {
    for (const install of [true, false])
      await h.test(`resolver-alias-action-${method}-${install}`, () => {
        const events = []
        const resolver = function (specifier, importer) {
          assert.equal(this, null)
          events.push([specifier, importer])
          return resolved
        }
        w.setModuleResolver(install ? null : resolver)
        const aliases = {
          get unused() {
            events.push('change')
            w.setModuleResolver(install ? resolver : null)
            return null
          },
        }
        const output =
          method === 'reserveClassSlots'
            ? reserve(w, [['src/alias-action.tsx', barrel, 'df']], aliases)
            : h.read(
                h.open({
                  mode: method,
                  code: barrel,
                  aliases,
                  file: 'src/alias-action.tsx',
                }),
              )
        assert.equal(events[0], 'change')
        assert.equal(events.some(Array.isArray), install)
        return { events, output }
      })
  }
  for (const mutation of [
    'replace',
    'clear',
    'resetBuildState',
    'resetStateForTesting',
  ]) {
    await h.test(`resolver-clone-${mutation}`, () => {
      const events = []
      const code = `import {Box} from '${pkg}'; import {color} from './one'; import {shade} from './two'; export const v=<Box color={color} bg={shade}/>`
      const replacement = function (specifier, importer) {
        assert.equal(this, null)
        events.push(['new', specifier, importer])
        return {
          path: `src/new-${specifier.slice(2)}.ts`,
          code: 'export const color="green"; export const shade="yellow";',
        }
      }
      w.setModuleResolver(function (specifier, importer) {
        assert.equal(this, null)
        events.push(['old', specifier, importer])
        if (events.length === 1) {
          switch (mutation) {
            case 'replace':
              w.setModuleResolver(replacement)
              break
            case 'clear':
              w.setModuleResolver(null)
              break
            case 'resetBuildState':
              w.resetBuildState()
              break
            case 'resetStateForTesting':
              w.resetStateForTesting()
              break
            default:
              assert.fail(mutation)
          }
        }
        return {
          path: `src/old-${specifier.slice(2)}.ts`,
          code: 'export const color="purple"; export const shade="blue";',
        }
      })
      const first = h.read(h.open({ code, file: 'src/current.tsx' }))
      assert.deepEqual(events, [
        ['old', './one', 'src/current.tsx'],
        ['old', './two', 'src/current.tsx'],
        ['old', './one', 'src/current.tsx'],
        ['old', './two', 'src/current.tsx'],
      ])
      const boundary = events.length
      const second = h.read(h.open({ code, file: 'src/next.tsx' }))
      assert.deepEqual(
        events.slice(boundary),
        mutation === 'replace'
          ? [
              ['new', './one', 'src/next.tsx'],
              ['new', './two', 'src/next.tsx'],
              ['new', './one', 'src/next.tsx'],
              ['new', './two', 'src/next.tsx'],
            ]
          : [],
      )
      return { events, first, second }
    })
  }
  await h.test('resolver-result-policy-path-code-this', () => {
    const observations = []
    for (const phase of [
      'invalid-path',
      'path-throw',
      'code-throw',
      'callback-throw',
      'valid',
    ]) {
      const events = []
      w.setModuleResolver(function () {
        assert.equal(this, null)
        events.push('callback')
        if (phase === 'callback-throw') throw undefined
        return {
          get path() {
            events.push('path')
            if (phase === 'path-throw') throw null
            return phase === 'invalid-path' ? 4 : resolved.path
          },
          get code() {
            events.push('code')
            if (phase === 'code-throw')
              throw {
                toString() {
                  assert.fail('Resolver throw must be forgiven without String')
                },
              }
            return resolved.code
          },
        }
      })
      assert.equal(
        w.hasDevupUI(`src/${phase}.tsx`, barrel, pkg, {}),
        phase === 'valid',
      )
      assert.deepEqual(events, [
        'callback',
        ...(phase === 'callback-throw' ? [] : ['path']),
        ...(['code-throw', 'valid'].includes(phase) ? ['code'] : []),
      ])
      observations.push([phase, events])
    }
    return observations
  })
  await runAliasConversions(h, barrel, resolved)
  await runResolverOrder(h)
  await runQueues(h)
}
