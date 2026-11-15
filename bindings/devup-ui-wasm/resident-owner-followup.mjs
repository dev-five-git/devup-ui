import assert from 'node:assert/strict'
import { test } from 'node:test'
import vm from 'node:vm'

import {
  api,
  caught,
  close,
  ownerSite as site,
  snapshot,
  wasm,
} from './resident-owner-fixtures.mjs'

const own = (context) => {
  const owner = api.create(snapshot)
  context.after(() => close(owner))
  return owner
}
const run = (owner, action) => api.run(owner, { site, configure() {}, action })
const chainContains = (error, original, seen = new Set()) => {
  if (error === original) return true
  if (!(error instanceof Error) || seen.has(error)) return false
  seen.add(error)
  return (
    chainContains(error.cause, original, seen) ||
    chainContains(error.cleanupCause, original, seen)
  )
}

test('F2 preserves an authentic Rust fault when a clean action rethrows it', (context) => {
  // Given the authentic Rust cause of an idle closed-owner refusal.
  const closed = own(context)
  api.dispose(closed)
  const original = caught(() => run(closed, () => {})).error.cause
  assert.ok(original instanceof Error)
  const owner = own(context)
  // When a clean live action throws that exact retained object.
  const { error } = caught(() =>
    run(owner, () => {
      throw original
    }),
  )
  // Then authentication does not authorize replacing a callback's thrown value.
  assert.equal(error, original)
})

for (const kind of ['closed', 'foreign']) {
  test(`F3 named ${kind} refusal reports the complete caller URL before request reads`, (context) => {
    // Given an inadmissible handle and a request which must remain unread.
    const owner = own(context)
    api.dispose(owner)
    let reads = 0
    const request = {
      get site() {
        reads++
        return site
      },
    }
    function invokeRefused() {
      return api.run(kind === 'closed' ? owner : {}, request)
    }
    // When a named caller enters the public API.
    const { error } = caught(invokeRefused)
    // Then the file component is the actual URL, without function label/parenthesis.
    const location = error.message.match(/^(.*):(\d+):(\d+): /)
    assert.ok(location)
    assert.equal(location[1], import.meta.url)
    assert.ok(Number(location[2]) > 0)
    assert.ok(Number(location[3]) > 0)
    assert.equal(reads, 0)
  })
}

test('F3 anonymous Windows frame preserves the complete drive path', (context) => {
  // Given a closed owner and an anonymous script with an actual Windows filename.
  const owner = own(context)
  api.dispose(owner)
  const filename = 'C:\\src\\anonymous-owner.js'
  // When the script invokes the API without reading a request.
  const { error } = caught(() =>
    vm.runInNewContext(
      'api.run(owner, undefined)',
      { api, owner },
      { filename },
    ),
  )
  // Then only the last two colon components are coordinates, not the drive colon.
  assert.equal(error.message.match(/^(.*):(\d+):(\d+): /)?.[1], filename)
})

test('F7 isolates suppressed row diagnostics from the next structural failure', (context) => {
  // Given distinct ordinary and structural resolver failures in a two-row batch.
  const owner = own(context)
  const first = new Error('ordinary row failure')
  const second = {
    get message() {
      caught(() => wasm.setPrefix('forbidden'))
      return 'structural row failure'
    },
  }
  const calls = { first: 0, second: 0 }
  const code =
    "import { Box } from '@devup-ui/react'; import { C } from './value'; export const x = <Box bg={C} />"
  // When row A is suppressed and row B catches a forbidden mutation during formatting.
  const { error } = caught(() =>
    run(owner, () => {
      wasm.setModuleResolver((_specifier, importer) => {
        if (importer === '/src/first-row.tsx') {
          calls.first++
          throw first
        }
        calls.second++
        throw second
      })
      wasm.reserveClassSlots(
        [
          ['/src/first-row.tsx', code, 'df'],
          ['/src/second-row.tsx', code, 'df'],
        ],
        '@devup-ui/react',
        true,
        false,
        false,
        {},
      )
    }),
  )
  // Then the final exported error retains only row B's cause and actual importer.
  assert.deepEqual(calls, { first: 1, second: 1 })
  assert.equal(chainContains(error, second), true)
  assert.equal(chainContains(error, first), false)
  assert.equal(error.message.match(/^(.*):1:1: /)?.[1], '/src/second-row.tsx')
})
