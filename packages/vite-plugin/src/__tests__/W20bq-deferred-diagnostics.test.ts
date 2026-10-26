import assert from 'node:assert/strict'

import { it } from 'bun:test'

import { DeferredDefaults } from '../deferred-defaults'
import { DeferredDefaultError } from '../deferred-metadata'
import { fixture, obligation, root } from './deferred-erasure-fixture'

it.each([undefined, '[]', JSON.stringify([obligation])])(
  'keeps own diagnostic locations and refusal classification when metadata is %s',
  async (metadata) => {
    // Given: own mapped diagnostics, with no fresh native evidence this epoch.
    const value = await fixture()
    value.begin()
    const serialized = JSON.stringify([obligation])
    value.deferred.extracted(value.environment, root, serialized, (prefix) => {
      assert.equal(prefix, `${root}:1:8`)
      return '/authored.mdx:5:8'
    })
    const graph = {
      ...value.graph(),
      getModuleInfo: () => ({ meta: { devupDeferredDefaults: metadata } }),
    }

    // When: verification rejects missing evidence or changed transport.
    // Then: only the location changes; the typed refusal and original read survive.
    assert.throws(
      () => value.deferred.verify(value.environment, graph),
      (error: unknown) => {
        assert(error instanceof DeferredDefaultError)
        assert.equal(error.name, 'DeferredDefaultError')
        assert.deepEqual(error.obligation, obligation)
        assert.equal(
          error.reason,
          metadata === serialized
            ? 'outer-result-unproven'
            : 'metadata-unproven',
        )
        assert.ok(error.message.startsWith('/authored.mdx:5:8:'))
        return true
      },
    )
  },
)

it('does not request diagnostic mapping when own obligations are empty', () => {
  // Given: an extraction with no deferred reads and a forbidden map lookup.
  const deferred = new DeferredDefaults()
  // When: that successful own extraction is recorded.
  // Then: no source map is requested for the empty contract.
  deferred.extracted({}, root, '[]', () =>
    assert.fail('Empty obligations must not request a source map'),
  )
})

it('does not adopt diagnostic prefixes from unowned public metadata', () => {
  // Given: unowned transport trying to supply its own diagnostic location.
  const deferred = new DeferredDefaults()
  const graph = {
    *getModuleIds() {
      yield root
    },
    getModuleInfo: () => ({
      meta: {
        devupDeferredDefaults: JSON.stringify([
          { ...obligation, diagnosticPrefix: '/forged.mdx:9:9' },
        ]),
      },
    }),
  }
  // When: the public metadata is rejected without an own extraction contract.
  // Then: it cannot impersonate an own-derived mapped diagnostic.
  assert.throws(
    () => deferred.verify({}, graph),
    (error: unknown) => {
      assert(error instanceof DeferredDefaultError)
      assert.equal(error.reason, 'metadata-unproven')
      assert.ok(error.message.startsWith(`${root}:1:8:`))
      return true
    },
  )
})
