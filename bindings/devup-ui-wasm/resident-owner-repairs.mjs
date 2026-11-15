import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { test } from 'node:test'

const wasm = createRequire(import.meta.url)('./pkg/index.js')
const api = wasm.ownership
const site = {
  file: '/src/repair.tsx',
  line: 4,
  column: 7,
  operation: 'extract',
}
const snapshot = () => ({
  sheet: wasm.exportSheet(),
  classes: wasm.exportClassMap(),
  files: wasm.exportFileMap(),
  canonical: wasm.exportCanonicalMap(),
  reservation: wasm.exportReservationState(),
})
const run = (owner, action) => api.run(owner, { site, configure() {}, action })
const caught = (action) => {
  try {
    action()
  } catch (error) {
    return error
  }
  throw new Error('expected failure')
}
const close = (owner) => {
  api.dispose(owner)
  api.clear(owner)
}
const own = (context, capture = snapshot) => {
  const owner = api.create(capture)
  context.after(() => close(owner))
  return owner
}
const literal = (color) =>
  `import { Box } from '@devup-ui/react'; export const x = <Box bg='${color}' />`
const imported =
  "import { Box } from '@devup-ui/react'; import { C } from './color'; export const x = <Box bg={C} />"
const extract = (code) =>
  wasm
    .codeExtract(
      '/src/repair.tsx',
      code,
      '@devup-ui/react',
      'df',
      true,
      false,
      false,
      {},
    )
    .free()
const chainContains = (error, original, seen = new Set()) => {
  if (error === original) return true
  if (!(error instanceof Error) || seen.has(error)) return false
  seen.add(error)
  return (
    chainContains(error.cause, original, seen) ||
    chainContains(error.cleanupCause, original, seen)
  )
}

for (const target of ['active', 'unrelated']) {
  test(`F1 ${target} disposal is checked before replacement publication without clear`, (context) => {
    // Given green CSS and two distinct owners.
    const owner = own(context)
    const other = own(context)
    run(owner, () => extract(literal('green')))
    // When resolution disposes only the selected owner, with no reset to hide publication.
    const action = () =>
      run(owner, () => {
        wasm.setModuleResolver(() => {
          api.dispose(target === 'active' ? owner : other)
          return { path: '/src/color.ts', code: "export const C = 'blue'" }
        })
        extract(imported)
      })
    if (target === 'active') assert.throws(action)
    else action()
    // Then active disposal cannot publish blue, while unrelated disposal permits it.
    const css = wasm.getCss(undefined, false)
    assert.match(
      css,
      target === 'active' ? /background:green/ : /background:blue/,
    )
    if (target === 'active') assert.doesNotMatch(css, /background:blue/)
  })
}

for (const kind of ['message', 'name', 'collision']) {
  test(`F2 action-only ${kind} Error retains exact identity`, (context) => {
    // Given a caller Error whose diagnostic properties are not trustworthy.
    const owner = own(context)
    const primary = new Error('primary')
    if (kind === 'collision') primary.name = 'ResidentOwnerError'
    else
      Object.defineProperty(primary, kind, {
        get() {
          throw new Error('hostile getter')
        },
      })
    // When the action alone fails and synchronous cleanup succeeds.
    const error = caught(() =>
      run(owner, () => {
        throw primary
      }),
    )
    // Then hostile diagnostics cannot substitute an error for the original value.
    assert.equal(error, primary)
  })
}

test('F1 resolver disposal rolls back scratch row allocations before reservation commit', (context) => {
  // Given an owner with no reserved class keys and a resolver-dependent reservation row.
  const owner = own(context)
  const before = run(owner, () => wasm.exportClassMap())
  // When its resolver disposes the active owner without requesting any reset.
  assert.throws(() =>
    run(owner, () => {
      wasm.setModuleResolver(() => {
        api.dispose(owner)
        return { path: '/src/color.ts', code: "export const C = 'blue'" }
      })
      wasm.reserveClassSlots(
        [['/src/repair.tsx', imported, 'df']],
        '@devup-ui/react',
        true,
        false,
        false,
        {},
      )
    }),
  )
  // Then scratch class allocation was not committed; FILE preseed is deliberately not rolled back.
  assert.equal(wasm.exportClassMap(), before)
})

test('F2 hostile combined primary keeps identity in cause without masking cleanup', (context) => {
  // Given a primary whose diagnostic getter catches reentry while admission must still be live.
  const owner = own(context)
  let reads = 0
  const primary = Object.defineProperty(new Error('primary'), 'message', {
    get() {
      reads++
      caught(() => run(owner, () => {}))
      throw new Error('hostile')
    },
  })
  // When action failure triggers that getter under the still-live synchronous admission.
  const error = caught(() =>
    run(owner, () => {
      throw primary
    }),
  )
  // Then both causes remain explicit and the primary is untouched.
  assert.equal(error.cause, primary)
  assert.ok(error.cleanupCause instanceof Error)
  assert.equal(reads, 1)
  assert.equal(Object.hasOwn(primary, 'cleanupCause'), false)
})

test('F3 idle foreign and disposed failures retain categories and actual callsites', (context) => {
  // Given a disposed local owner and a foreign object, without active work to finish.
  const owner = own(context)
  api.dispose(owner)
  // When either is used from this real source callsite.
  const closed = caught(() => run(owner, () => {}))
  const foreign = caught(() => run({}, () => {}))
  // Then neither claims reentry recovery and both are located at the caller.
  for (const [error, category] of [
    [closed, 'ClosedOwner'],
    [foreign, 'ForeignEngine'],
  ]) {
    assert.match(error.message, /resident-owner-repairs\.mjs:\d+:\d+:/)
    assert.match(error.message, new RegExp(category))
    assert.doesNotMatch(error.message, /finish the current synchronous/)
  }
})

test('F5 capture primary retains distinct caught foreign cleanup refusal', (context) => {
  // Given an outgoing capture which catches a foreign rejection then throws its own primary.
  const primary = new Error('capture primary')
  const a = own(context, () => {
    caught(() => run({}, () => {}))
    throw primary
  })
  const b = own(context)
  run(a, () => extract(literal('red')))
  // When preservation fails on takeover.
  const error = caught(() => run(b, () => {}))
  // Then the separate structural failure is not silently discarded.
  assert.ok(chainContains(error, primary))
  assert.ok(error.cleanupCause instanceof Error)
  assert.match(error.cleanupCause.message, /ForeignEngine/)
})

test('F6 direct clear drains deferred reset before the first innocent read', (context) => {
  // Given an idle resident whose capture disposes itself and queues targeted cleanup.
  const owner = own(context, () => {
    const value = snapshot()
    api.dispose(owner)
    api.clear(owner)
    return value
  })
  run(owner, () => extract(literal('red')))
  // When the direct clear fails after the capture frame unwinds.
  assert.throws(() => api.clear(owner))
  // Then the very first ordinary read succeeds against the already drained bank.
  assert.doesNotMatch(wasm.getCss(undefined, false), /background:red/)
})

for (const operation of ['hasDevupUI', 'reserveClassSlots']) {
  test(`F7 ${operation} retains original resolver cause and transitive importer`, (context) => {
    // Given a transitive resolver exception whose message getter catches reentry.
    const owner = own(context)
    const original = {
      get message() {
        caught(() => wasm.setPrefix('forbidden'))
        return 'original resolver failure'
      },
    }
    const code =
      operation === 'hasDevupUI'
        ? "import { Box } from './barrel'; export const x = <Box bg='red' />"
        : "import { Box } from '@devup-ui/react'; import { C } from './barrel'; export const x = <Box bg={C} />"
    // When the second resolver edge fails during structural error formatting.
    const error = caught(() =>
      run(owner, () => {
        wasm.setModuleResolver((specifier) => {
          if (specifier === './barrel')
            return {
              path: '/src/transitive.ts',
              code: "export { Box, C } from './broken'",
            }
          throw original
        })
        if (operation === 'hasDevupUI')
          wasm.hasDevupUI('/src/repair.tsx', code, '@devup-ui/react', {})
        else
          wasm.reserveClassSlots(
            [['/src/repair.tsx', code, 'df']],
            '@devup-ui/react',
            true,
            false,
            false,
            {},
          )
      }),
    )
    // Then the final structural error, not merely an inner caught error, retains origin/cause.
    assert.ok(chainContains(error, original))
    assert.match(error.message, /^\/src\/transitive\.ts:1:1:/)
  })
}
