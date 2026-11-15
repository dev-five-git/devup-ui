import assert from 'node:assert/strict'
import { test } from 'node:test'

import {
  api,
  caught,
  close,
  snapshot,
  wasm,
} from './resident-owner-fixtures.mjs'

const site = {
  file: '/src/errors.tsx',
  line: 2,
  column: 3,
  operation: 'extract',
}
const run = (owner, action, configure = () => {}) =>
  api.run(owner, { site, configure, action })
const source =
  "import { Box } from '@devup-ui/react'; import { C } from './color'; export const x = <Box bg={C} />"
const extract = (code = source) => {
  const output = wasm.codeExtract(
    '/src/errors.tsx',
    code,
    '@devup-ui/react',
    'df',
    true,
    false,
    false,
    {},
  )
  output.free()
}

for (const original of [new Error('resolver-original'), undefined]) {
  test(`resolver retains original cause when it throws ${original === undefined ? 'undefined' : 'an Error'}`, () => {
    // Given a resolver throwing an exact original value.
    const owner = api.create(snapshot)
    // When extraction surfaces that boundary failure.
    const { error } = caught(() =>
      run(owner, () => {
        wasm.setModuleResolver(() => {
          throw original
        })
        extract()
      }),
    )
    // Then the thrown value remains the cause even when undefined.
    assert.equal(Object.hasOwn(error, 'cause'), true)
    assert.equal(error.cause, original)
    close(owner)
  })
}

test('capture failure faults outgoing owner before incoming activation', () => {
  // Given an outgoing owner whose capture throws undefined.
  const a = api.create(() => {
    throw undefined
  })
  const b = api.create(snapshot)
  run(a, () =>
    extract(
      "import { Box } from '@devup-ui/react'; export const x = <Box bg='red' />",
    ),
  )
  let action = false
  // When B requests takeover.
  const { error } = caught(() =>
    run(b, () => {
      action = true
    }),
  )
  // Then capture's exact cause is retained and B never executes.
  assert.equal(Object.hasOwn(error, 'cause'), true)
  assert.equal(error.cause, undefined)
  assert.equal(action, false)
  assert.equal(a.disposed, true)
  close(a)
  close(b)
})

test('configuration failure aborts matching phase without admitting action', () => {
  // Given configuration which fails after one real export.
  const owner = api.create(snapshot)
  const primary = new Error('configure-original')
  let action = false
  // When configuration throws.
  const { error } = caught(() =>
    run(
      owner,
      () => {
        action = true
      },
      () => {
        wasm.setPrefix('configured')
        throw primary
      },
    ),
  )
  // Then abort retains the primary and releases the bank for a fresh owner.
  assert.equal(error.cause, primary)
  assert.equal(action, false)
  assert.equal(owner.disposed, true)
  close(owner)
  const fresh = api.create(snapshot)
  assert.equal(
    run(fresh, () => 7),
    7,
  )
  close(fresh)
})

test('open targeted clear preserves its owner and obsolete cleanup skips another bank', () => {
  // Given A with live CSS, B with different CSS, and a preserved A checkpoint.
  const a = api.create(snapshot)
  const b = api.create(snapshot)
  run(a, () =>
    extract(
      "import { Box } from '@devup-ui/react'; export const x = <Box bg='red' />",
    ),
  )
  api.clear(a)
  assert.equal(a.disposed, false)
  run(b, () =>
    extract(
      "import { Box } from '@devup-ui/react'; export const x = <Box bg='blue' />",
    ),
  )
  // When obsolete A cleanup is requested while B owns the physical bank.
  api.clear(a)
  // Then neither B's bank nor A's earlier recoverable checkpoint was erased.
  assert.match(
    run(b, () => wasm.getCss(undefined, false)),
    /background:blue/,
  )
  assert.match(
    run(a, () => wasm.getCss(undefined, false)),
    /background:red/,
  )
  close(a)
  close(b)
})

test('public testing reset preserves coordinator identities and saved checkpoints', () => {
  // Given A with a saved checkpoint and B resident.
  const a = api.create(snapshot)
  const b = api.create(snapshot)
  run(a, () =>
    extract(
      "import { Box } from '@devup-ui/react'; export const x = <Box bg='purple' />",
    ),
  )
  run(b, () => {})
  // When the public testing reset mutates the raw bank.
  wasm.resetStateForTesting()
  // Then A's handle and matching saved state remain authoritative.
  assert.equal(a.disposed, false)
  assert.match(
    run(a, () => wasm.getCss(undefined, false)),
    /background:purple/,
  )
  close(a)
  close(b)
})

test('caught resolver reentry cannot publish a replacement sheet', () => {
  // Given an existing contribution and a resolver which catches read reentry.
  const owner = api.create(snapshot)
  run(owner, () =>
    extract(
      "import { Box } from '@devup-ui/react'; export const x = <Box bg='green' />",
    ),
  )
  let nested
  // When re-extraction encounters that resolver.
  assert.throws(() =>
    run(owner, () => {
      wasm.setModuleResolver(() => {
        nested = caught(() => wasm.getCss(undefined, false)).error
        return { path: '/src/color.ts', code: "export const C = 'blue'" }
      })
      extract()
    }),
  )
  // Then importer coordinates are real and the previous contribution survives.
  assert.match(nested.message, /^\/src\/errors\.tsx:1:1:/)
  const css = run(owner, () => wasm.getCss(undefined, false))
  assert.match(css, /background:green/)
  assert.doesNotMatch(css, /background:blue/)
  close(owner)
})

test('committed checkpoint is independent of later capture envelope mutation', () => {
  // Given A whose capture envelope remains mutable in host code.
  let envelope
  const a = api.create(() => {
    envelope = snapshot()
    return envelope
  })
  const b = api.create(snapshot)
  run(a, () =>
    extract(
      "import { Box } from '@devup-ui/react'; export const x = <Box bg='salmon' />",
    ),
  )
  run(b, () => {})
  // When host code corrupts every retained wire field after commit.
  for (const key of Object.keys(envelope)) envelope[key] = 'invalid'
  // Then A restores its parsed immutable value without reading that object again.
  assert.match(
    run(a, () => wasm.getCss(undefined, false)),
    /background:salmon/,
  )
  close(a)
  close(b)
})

test('exception formatting retains original cause while caught reentry latches', () => {
  // Given a resolver exception whose message getter catches forbidden engine work.
  const owner = api.create(snapshot)
  const original = {
    get message() {
      caught(() => wasm.setPrefix('forbidden'))
      return 'host failure'
    },
  }
  // When the resolver exception is formatted under admission.
  const { error } = caught(() =>
    run(owner, () => {
      wasm.setModuleResolver(() => {
        throw original
      })
      extract()
    }),
  )
  // Then the exact original survives in the error chain and success is impossible.
  let current = error
  const seen = new Set()
  while (
    current !== original &&
    current instanceof Error &&
    !seen.has(current)
  ) {
    seen.add(current)
    current = current.cause
  }
  assert.equal(current, original)
  close(owner)
})

test('disposal during capture aborts preservation and drains targeted cleanup', () => {
  // Given an outgoing capture which closes itself while its callback is active.
  let captures = 0
  const a = api.create(() => {
    captures++
    const value = snapshot()
    api.dispose(a)
    api.clear(a)
    return value
  })
  const b = api.create(snapshot)
  run(a, () =>
    extract(
      "import { Box } from '@devup-ui/react'; export const x = <Box bg='red' />",
    ),
  )
  // When B attempts the takeover.
  assert.throws(() => run(b, () => {}))
  // Then no disposed capture is committed/reinvoked and B can subsequently enter.
  assert.equal(a.disposed, true)
  assert.equal(
    run(b, () => 17),
    17,
  )
  assert.equal(captures, 1)
  close(b)
})
