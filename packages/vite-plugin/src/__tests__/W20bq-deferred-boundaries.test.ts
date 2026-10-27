import assert from 'node:assert/strict'

import { expect, it } from 'bun:test'

import { DeferredDefaults } from '../deferred-defaults'
import { DeferredMetadataError } from '../deferred-metadata'
import { OuterResolution } from '../outer-resolution'
import { fixture } from './deferred-erasure-fixture'

it('retains final facts when observation arrives before explicit start', async () => {
  // Given: current owned edges and final facts, but no explicit module-map start.
  const value = await fixture(false)
  // When: verification needs both lazily observed root and target facts.
  // Then: the owned import-only cut succeeds rather than losing those facts.
  expect(() =>
    value.deferred.verify(value.environment, value.graph()),
  ).not.toThrow()
})

it.each([
  ['non-array', '{}', TypeError],
  ['invalid record', '[null]', TypeError],
  ['invalid JSON', '{', SyntaxError],
] as const)(
  'preserves the parser cause when unowned metadata is %s',
  (_name, metadata, kind) => {
    // Given: native module metadata with no matching extraction contract.
    const deferred = new DeferredDefaults()
    const graph = {
      *getModuleIds() {
        yield '/entry.js'
      },
      getModuleInfo() {
        return { meta: { devupDeferredDefaults: metadata } }
      },
    }
    // When: verification reads the actual metadata text boundary.
    let failure: unknown
    try {
      deferred.verify({}, graph)
    } catch (cause) {
      failure = cause
    }
    // Then: the located refusal retains the real SyntaxError/TypeError.
    expect(failure).toBeInstanceOf(DeferredMetadataError)
    assert(failure instanceof DeferredMetadataError)
    expect(failure.filename).toBe('/entry.js')
    expect(failure.cause).toBeInstanceOf(kind)
  },
)

it.each(['missing', 'object-hook', 'duplicate'] as const)(
  'does not install resolution authority when the terminal is %s',
  (kind) => {
    // Given: actual installer input without exactly one callable terminal.
    const resolution = new OuterResolution()
    const companion = resolution.companion()
    const environment = {}
    const terminal = { name: 'terminal', api: companion.api, transform() {} }
    const candidates = {
      missing: [{ name: 'other' }],
      'object-hook': [{ ...terminal, transform: { handler() {} } }],
      duplicate: [terminal, terminal],
    }
    assert(typeof companion.options === 'function')
    // When: the real options hook considers the unsupported installation.
    const installed: unknown = Reflect.apply(
      companion.options,
      { environment },
      [{ plugins: candidates[kind] }],
    )
    // Then: no owner is returned and no environment gains authority.
    expect(installed).toBeUndefined()
    expect(resolution.valid(environment)).toBe(false)
  },
)
