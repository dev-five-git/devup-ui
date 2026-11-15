import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { test } from 'node:test'
import { fileURLToPath } from 'node:url'
import vm from 'node:vm'

import {
  api,
  caught,
  close,
  ownerSite as site,
  snapshot,
  wasm,
} from './resident-owner-fixtures.mjs'

const require = createRequire(import.meta.url)
const filename = fileURLToPath(new URL('./pkg/index.js', import.meta.url))
const esm = await import('./pkg/index.js')
const run = (owner, action, configure = () => {}) =>
  api.run(owner, { site, configure, action })
const extract = (color, method = 'codeExtract') => {
  const output = wasm[method](
    '/src/owner.tsx',
    `import { Box } from '@devup-ui/react'; export const x = <Box bg='${color}' />`,
    '@devup-ui/react',
    'df',
    true,
    false,
    false,
    {},
  )
  output.free()
}

test('CJS and ESM expose the same guarded surface without controls', () => {
  // Given real CJS and ESM loads of the rebuilt artifact.
  // When consumers enumerate callable exports.
  const keys = Object.keys(wasm)
  // Then controls are private and named ESM discovery preserves public functions.
  assert.equal(
    keys.some((name) => name.startsWith('_owner')),
    false,
  )
  assert.equal(esm.seedFileMap, wasm.seedFileMap)
  assert.equal(esm.ownership, api)
})

test('same resident does not capture eagerly and A/B/A restores real CSS', () => {
  // Given two owners and distinguishing live stylesheet contributions.
  let captures = 0
  const a = api.create(() => {
    captures++
    return snapshot()
  })
  const b = api.create(snapshot)
  run(a, () => extract('red'))
  run(a, () => assert.match(wasm.getCss(undefined, false), /background:red/))
  assert.equal(captures, 0)
  // When B takes over and A resumes.
  run(b, () => extract('blue'))
  const css = run(a, () => wasm.getCss(undefined, false))
  // Then one outgoing A capture restored A, not B.
  assert.equal(captures, 1)
  assert.match(css, /background:red/)
  assert.doesNotMatch(css, /background:blue/)
  close(a)
  close(b)
})

test('raw mutation preserves the resident before changing live globals', () => {
  // Given a resident with real CSS.
  let captures = 0
  const owner = api.create(() => {
    captures++
    return snapshot()
  })
  run(owner, () => extract('tomato'))
  // When an unmanaged destructive reset is admitted.
  wasm.resetBuildState()
  // Then the owner restores its outgoing state rather than empty globals.
  assert.equal(captures, 1)
  assert.match(
    run(owner, () => wasm.getCss(undefined, false)),
    /background:tomato/,
  )
  close(owner)
})

for (const kind of ['array', 'string', 'object']) {
  test(`caught ${kind} marshalling reentry latches the enclosing export`, () => {
    // Given a hostile conversion getter which catches the inner refusal.
    const owner = api.create(snapshot)
    let inner
    const reenter = () => {
      inner = caught(() => wasm.setPrefix('forbidden')).error
    }
    const action =
      kind === 'array'
        ? () =>
            wasm.seedFileMap({
              get length() {
                reenter()
                return 0
              },
            })
        : kind === 'string'
          ? () =>
              wasm.setPrefix({
                get length() {
                  reenter()
                  return 0
                },
              })
          : () =>
              wasm.importCanonicalMap({
                get value() {
                  reenter()
                  return 'other'
                },
              })
    // When conversion attempts the nested mutation.
    const failure = caught(() => run(owner, action)).error
    // Then catching it cannot publish success or the forbidden prefix.
    assert.ok(inner instanceof Error)
    assert.ok(failure instanceof Error)
    assert.notEqual(
      run(owner, () => wasm.getPrefix()),
      'forbidden',
    )
    close(owner)
  })
}

test('action throw undefined is preserved when cleanup succeeds', () => {
  // Given an owner whose action throws the undefined value.
  const owner = api.create(snapshot)
  // When synchronous action unwinds.
  const failure = caught(() =>
    run(owner, () => {
      throw undefined
    }),
  )
  // Then failure presence is retained independently of its value.
  assert.equal(failure.error, undefined)
  close(owner)
})

test('combined errors preserve the primary and cleanup causes', () => {
  // Given a primary error and a caught nested ownership violation.
  const owner = api.create(snapshot)
  const primary = new Error('primary-action')
  // When both action and latched unwind fail.
  const { error } = caught(() =>
    run(owner, () => {
      caught(() => run(owner, () => {}))
      throw primary
    }),
  )
  // Then the caller error is not mutated or masked.
  assert.equal(error.cause, primary)
  assert.ok(error.cleanupCause instanceof Error)
  assert.equal(Object.hasOwn(primary, 'cleanupCause'), false)
  close(owner)
})

test('foreign instance handles fail before request getters', () => {
  // Given an independent physical WASM instance and its coinciding local owner number.
  const exports = {}
  const load = vm.runInThisContext(
    `(function(require,module,exports,__filename,__dirname){${readFileSync(filename, 'utf8')}\n})`,
    { filename },
  )
  load(
    require,
    { exports },
    exports,
    filename,
    fileURLToPath(new URL('./pkg/', import.meta.url)),
  )
  const foreign = exports.ownership.create(() => {
    throw new Error('unused')
  })
  let reads = 0
  // When the local bridge receives the foreign object.
  assert.throws(() =>
    api.run(foreign, {
      get site() {
        reads++
        return site
      },
    }),
  )
  // Then no request property or local bank transition ran.
  assert.equal(reads, 0)
  exports.ownership.dispose(foreign)
})

test('disposal during a resolver defers targeted reset until extraction unwinds', () => {
  // Given a live owner with an executing resolver frame.
  const owner = api.create(snapshot)
  run(owner, () => extract('green'))
  const source =
    "import { Box } from '@devup-ui/react'; import { C } from './color'; export const x = <Box bg={C} />"
  // When disposal and clear happen in that frame.
  assert.throws(() =>
    run(owner, () => {
      wasm.setModuleResolver(() => {
        api.dispose(owner)
        api.clear(owner)
        return { path: '/src/color.ts', code: "export const C = 'blue'" }
      })
      wasm.codeExtract(
        '/src/owner.tsx',
        source,
        '@devup-ui/react',
        'df',
        true,
        false,
        false,
        {},
      )
    }),
  )
  // Then the closed owner cannot publish and the drained bank contains no old CSS.
  assert.equal(owner.disposed, true)
  assert.doesNotMatch(
    wasm.getCss(undefined, false),
    /background:(?:green|blue)/,
  )
})

test('Promise return releases execution synchronously', async () => {
  // Given an action returning a still-pending Promise.
  const owner = api.create(snapshot)
  let resolve
  const promise = new Promise((complete) => {
    resolve = complete
  })
  // When another synchronous admission occurs before resolution.
  const returned = run(owner, () => promise)
  const value = run(owner, () => 42)
  // Then execution was not held across the await.
  assert.equal(returned, promise)
  assert.equal(value, 42)
  resolve()
  await returned
  close(owner)
})

for (const operation of ['hasDevupUI', 'reserveClassSlots']) {
  test(`${operation} cannot suppress a resolver ownership fault`, () => {
    // Given a resolver which catches nested mutation and returns valid source.
    const owner = api.create(snapshot)
    const code =
      operation === 'hasDevupUI'
        ? "import { Box } from './color'; export const x = <Box bg='red' />"
        : "import { Box } from '@devup-ui/react'; import { C } from './color'; export const x = <Box bg={C} />"
    let calls = 0
    // When the relevance/reservation path invokes the resolver.
    assert.throws(() =>
      run(owner, () => {
        wasm.setModuleResolver(() => {
          calls++
          caught(() => wasm.resetBuildState())
          return {
            path: '/src/color.ts',
            code: "export { Box } from '@devup-ui/react'; export const C = 'red'",
          }
        })
        if (operation === 'hasDevupUI')
          wasm.hasDevupUI('/src/owner.tsx', code, '@devup-ui/react', {})
        else
          wasm.reserveClassSlots(
            [['/src/owner.tsx', code, 'df']],
            '@devup-ui/react',
            true,
            false,
            false,
            {},
          )
      }),
    )
    // Then the outer operation fails and scratch frames have released for cleanup.
    assert.ok(calls > 0)
    close(owner)
    const fresh = api.create(snapshot)
    run(fresh, () => extract('orange', 'codeExtractWithoutSourceMap'))
    close(fresh)
  })
}
