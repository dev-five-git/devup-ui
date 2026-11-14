import assert from 'node:assert/strict'

import { pkg } from './corpus/extraction.mjs'
import {
  indexed,
  iteratorFailure,
  iteratorPhases,
  runAliasRaw,
  slot,
  wrappedPhases,
} from './entries-fixtures.mjs'
import { runSlots } from './entries-slots.mjs'

export async function runErrors(h) {
  const { w } = h
  for (const kind of ['map', 'enum']) {
    for (const phase of [
      'lookup',
      'call',
      'enumeration',
      'length',
      'coercion',
    ]) {
      for (const sentinelName of ['undefined', 'null', 'hostile']) {
        await h.test(`${kind}-raw-${phase}-${sentinelName}`, () => {
          const fixture = h.fixture(kind),
            events = []
          let reads = 0
          let armed = kind === 'map'
          if (kind === 'enum')
            fixture.prepare(() => {
              armed = true
            })
          const sentinel =
            sentinelName === 'undefined'
              ? undefined
              : sentinelName === 'null'
                ? null
                : {
                    [Symbol.toPrimitive]() {
                      assert.fail('Raw throw must never be stringified')
                    },
                  }
          const fail = () => {
            events.push(phase)
            throw sentinel
          }
          if (phase === 'enumeration')
            Object.defineProperty(
              fixture.target,
              kind === 'map' ? 'a' : fixture.pair[0],
              { enumerable: true, configurable: true, get: fail },
            )
          const entries = indexed((_value, key) => {
            assert.equal(
              key,
              'length',
              'Failure must short-circuit before index',
            )
            if (kind === 'enum' && ++reads === 1) return 0
            if (phase === 'length') return fail()
            return { valueOf: fail }
          })
          const original = Object.entries
          const descriptor =
            phase === 'lookup'
              ? {
                  get() {
                    return armed ? fail() : original
                  },
                }
              : {
                  value(value) {
                    if (value !== fixture.target)
                      return Reflect.apply(original, this, [value])
                    if (phase === 'call') return fail()
                    if (phase === 'enumeration') return original(value)
                    return entries
                  },
                }
          h.raw(
            () => h.patch(descriptor, () => fixture.invoke(fixture.target)),
            sentinel,
          )
          assert.deepEqual(events, [phase])
          return events
        })
      }
    }
  }
  for (const hostile of [false, true]) {
    const phases = hostile ? wrappedPhases : iteratorPhases
    const group = hostile ? 'iterator-hostile-String' : 'iterator-error'
    for (const phase of phases) {
      await h.test(`${group}-${phase}`, () => {
        const events = [],
          conversionFailure = {}
        const sentinel = {
          toString() {
            events.push('String')
            if (hostile) throw conversionFailure
            return 'iterator sentinel'
          },
        }
        const fail = () => {
          events.push(phase)
          throw sentinel
        }
        const target = iteratorFailure(phase, fail)
        if (hostile) {
          h.raw(() => w.importFileMap(target), conversionFailure)
          assert.deepEqual(events, [phase, 'String'])
        } else {
          const result = h.take(() => w.importFileMap(target))
          assert.equal(result.caught, true)
          const wrapped = wrappedPhases.includes(phase)
          if (wrapped) {
            assert.equal(result.error.constructor, Error)
            assert.equal(result.error.message, 'iterator sentinel')
          } else assert.equal(result.error, sentinel)
          assert.deepEqual(events, [phase, ...(wrapped ? ['String'] : [])])
        }
        return events
      })
    }
  }
  await h.test('iterators-bypass-entries-and-fallback-shapes', () => {
    const observations = []
    for (const value of [
      new Map([['a', 2]]),
      {
        *[Symbol.iterator]() {
          yield ['a', 2]
        },
      },
    ]) {
      h.patch(
        {
          get() {
            assert.fail('Iterable must bypass entries and E.length')
          },
        },
        () => w.importFileMap(value),
      )
      assert.deepEqual(JSON.parse(w.exportFileMap()), { a: 2 })
    }
    for (const method of [
      7,
      () => null,
      () => 3,
      () => ({ next: 7 }),
      () =>
        Object.assign(function iterator() {}, {
          next() {
            assert.fail('Callable iterator is rejected')
          },
        }),
    ]) {
      const value = { a: 3, [Symbol.iterator]: method }
      w.importFileMap(value)
      observations.push(w.exportFileMap())
      assert.deepEqual(JSON.parse(w.exportFileMap()), { a: 3 })
    }
    const callable = Object.assign(function source() {}, { a: 3 })
    h.schema(() => w.importFileMap(callable), slot.file)
    callable[Symbol.iterator] = function* () {
      yield ['a', 6]
    }
    w.importFileMap(callable)
    assert.deepEqual(JSON.parse(w.exportFileMap()), { a: 6 })
    return observations
  })
  await runAliasRaw(h)
  await runSlots(h)
}

export async function runNamespaceOrder(h) {
  const { w } = h
  await h.test('namespace-barrel-lexical-callback-subset', () => {
    const events = []
    const modules = {
      './barrel': {
        path: 'src/barrel.ts',
        code: "export * as ns from './namespace';",
      },
      './namespace': {
        path: 'src/namespace.ts',
        code: "export { Box as alpha } from './left'; export { Box as beta } from './right';",
      },
      './left': {
        path: 'src/left.ts',
        code: `export { Box } from '${pkg}';`,
      },
      './right': {
        path: 'src/right.ts',
        code: `export { Box } from '${pkg}';`,
      },
    }
    w.setModuleResolver((specifier, importer) => {
      events.push([specifier, importer])
      assert.ok(Object.hasOwn(modules, specifier), specifier)
      return modules[specifier]
    })
    const result = h.take(() =>
      h.read(
        h.open({
          file: 'src/W74l-entry.tsx',
          code: "import { ns } from './barrel'; export const view = <ns.Box p={2}/>;",
        }),
      ),
    )
    assert.equal(result.caught, true)
    assert.ok(result.error instanceof Error)
    assert.equal(
      result.error.message,
      'src/W74l-entry.tsx:1:53: `ns` cannot use `./barrel` at build time: the build cannot follow its re-export of Devup UI (`./namespace` is a namespace of modules re-exporting `@devup-ui/react`); import it from the package instead',
    )
    assert.deepEqual(events, [
      ['./barrel', 'src/W74l-entry.tsx'],
      ['./namespace', 'src/barrel.ts'],
      ['./left', 'src/namespace.ts'],
    ])
    return { events, message: result.error.message }
  })
  await h.test('namespace-change-lowest-object-key', () => {
    const events = []
    w.setModuleResolver((specifier, importer) => {
      events.push([specifier, importer])
      assert.equal(specifier, './tokens')
      return {
        path: 'src/tokens.ts',
        code: "export const alpha = { color: 'red' };\nexport const beta = { color: 'blue' };\nalpha.color = 'pink';\nbeta.color = 'cyan';",
      }
    })
    const result = h.take(() =>
      h.read(
        h.open({
          file: 'src/W74l-entry.tsx',
          code: "import { css } from '@devup-ui/react'; import * as tokens from './tokens'; export const cls = css(tokens);",
        }),
      ),
    )
    assert.equal(result.caught, true)
    assert.ok(result.error instanceof Error)
    assert.equal(
      result.error.message,
      'src/W74l-entry.tsx:1:99: `css()` cannot use `tokens` at build time: its styles must be an object literal or a constant object, or be computed from constants\nsrc/tokens.ts:3:1: `alpha` is changed here, so the build cannot read it as a constant',
    )
    const callback = ['./tokens', 'src/W74l-entry.tsx']
    assert.deepEqual(
      events,
      Array.from({ length: 5 }, () => callback),
    )
    return { events, message: result.error.message }
  })
}
