import { expect, it } from 'bun:test'

import { DeferredDefaults } from '../deferred-defaults'
import { fixture, obligation, root } from './deferred-erasure-fixture'

it.each([undefined, '[]', 0])(
  'retains located debt when native transport changes to %s',
  async (metadata) => {
    // Given a real owned contract followed by a new epoch with no fresh evidence.
    const value = await fixture()
    value.begin()
    const graph = {
      ...value.graph(),
      getModuleInfo: () => ({ meta: { devupDeferredDefaults: metadata } }),
    }
    // When native transport is missing or replaced, Then prior debt still refuses located.
    expect(() => value.deferred.verify(value.environment, graph)).toThrow(
      /page\.tsx:1:8:.*metadata-unproven/,
    )
  },
)

it('keeps absent module debt dormant until a genuine own empty extraction replaces it', async () => {
  const value = await fixture()
  // Given an owned module absent from this graph, When it later reappears unchanged.
  value.deferred.verify(value.environment, {
    *getModuleIds() {},
    getModuleInfo: value.graph().getModuleInfo,
  })
  const graph = {
    ...value.graph(),
    getModuleInfo: () => ({ meta: { devupDeferredDefaults: '[]' } }),
  }
  expect(() => value.deferred.verify(value.environment, graph)).toThrow(
    /metadata-unproven/,
  )
  // Then only a successful own empty result retires the previous obligation.
  value.deferred.extracted(value.environment, root, '[]')
  expect(() => value.deferred.verify(value.environment, graph)).not.toThrow()
  expect(() => value.deferred.verify(value.environment, value.graph())).toThrow(
    /page\.tsx:1:1: deferred default metadata/,
  )
})

it.each([JSON.stringify([obligation]), '[]', 0, '{', '[null]'])(
  'does not adopt unowned public metadata %s',
  (metadata) => {
    const deferred = new DeferredDefaults()
    const graph = {
      *getModuleIds() {
        yield root
      },
      getModuleInfo: () => ({ meta: { devupDeferredDefaults: metadata } }),
    }
    expect(() => deferred.verify({}, graph)).toThrow(
      /page\.tsx:\d+:\d+: deferred default/,
    )
  },
)
