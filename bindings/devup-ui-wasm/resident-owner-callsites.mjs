import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname } from 'node:path'
import { test } from 'node:test'
import { fileURLToPath } from 'node:url'
import vm from 'node:vm'

import {
  api,
  caught,
  close,
  ownerSite as site,
  snapshot,
} from './resident-owner-fixtures.mjs'

const callers = [
  'function invokeRefused() { return api.run(handle, request) } invokeRefused()',
  "const caller = { 'label (copy)'() { return api.run(handle, request) } }; caller['label (copy)']()",
  'api.run(handle, request)',
]
for (const filename of [
  'C:\\src\\owner (copy)\\caller.js',
  'C:\\src\\caller.js',
  'file:///src/owner (copy)/caller.js',
]) {
  for (const [index, source] of callers.entries()) {
    for (const kind of ['closed', 'foreign']) {
      test(`${kind} caller ${index} retains entire ${filename}`, (context) => {
        // Given an inadmissible handle and observable request getters.
        const owner = api.create(snapshot)
        context.after(() => close(owner))
        api.dispose(owner)
        let reads = 0
        const request = new Proxy(
          {},
          {
            get() {
              reads++
              throw new Error('request must remain unread')
            },
          },
        )
        // When a real named/labelled/anonymous VM frame calls the generated API.
        const { error } = caught(() =>
          vm.runInNewContext(
            source,
            { api, handle: kind === 'closed' ? owner : {}, request },
            { filename },
          ),
        )
        // Then the complete file is preserved without labels or wrapper parentheses.
        const location = error.message.match(/^(.*):(\d+):(\d+): /)
        assert.ok(location)
        assert.equal(location[1], filename)
        assert.ok(Number(location[2]) > 0)
        assert.ok(Number(location[3]) > 0)
        assert.equal(reads, 0)
      })
    }
  }
}

test('successful explicit-site resident runs construct no Error', (context) => {
  // Given an isolated real generated JS/WASM instance with counted Error construction.
  const filename = fileURLToPath(new URL('./pkg/index.js', import.meta.url))
  let constructions = 0
  class CountingError extends Error {
    constructor(...args) {
      super(...args)
      constructions++
    }
  }
  const exports = {}
  vm.runInNewContext(
    readFileSync(filename, 'utf8'),
    {
      exports,
      module: { exports },
      require: createRequire(filename),
      __filename: filename,
      __dirname: dirname(filename),
      Error: CountingError,
      TextDecoder,
      TextEncoder,
    },
    { filename },
  )
  const wasm = exports
  const ownership = wasm.ownership
  const owner = ownership.create(() => ({
    sheet: wasm.exportSheet(),
    classes: wasm.exportClassMap(),
    files: wasm.exportFileMap(),
    canonical: wasm.exportCanonicalMap(),
    reservation: wasm.exportReservationState(),
  }))
  context.after(() => {
    ownership.dispose(owner)
    ownership.clear(owner)
  })
  const request = {
    site,
    configure() {
      wasm.setPrefix('counted')
    },
    action() {
      return wasm.getPrefix()
    },
  }
  ownership.run(owner, request)
  const before = constructions
  // When the valid explicit-site resident path executes configuration and an export.
  const values = Array.from({ length: 3 }, () => ownership.run(owner, request))
  const constructed = constructions - before
  // Then the mechanism count, not elapsed time, proves no eager fallback stack.
  assert.deepEqual(values, ['counted', 'counted', 'counted'])
  assert.equal(constructed, 0)
  context.diagnostic(
    `successful resident runs=3; Error constructions=${constructed}`,
  )
})
