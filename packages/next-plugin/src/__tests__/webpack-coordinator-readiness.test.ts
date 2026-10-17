import { expect, it } from 'bun:test'

import { compiler, fixture } from './webpack-bridge-fixture'

it.each(['ready', 'prepared'] as const)(
  'retains the original located failure when coordinator %s rejects',
  async (gate) => {
    // Given
    const cause = new Error('/app/page.mdx:3:8: preparation failed')
    const pending = Promise.withResolvers<void>()
    const { integration } = fixture()
    const { compose } = fixture({
      handle: { ...integration.handle, [gate]: pending.promise },
    })
    const instance = compiler(compose({}))
    // When
    const work = instance.hooks.beforeCompile.promise(
      instance.newCompilationParams(),
    )
    pending.reject(cause)
    // Then
    await expect(work).rejects.toBe(cause)
  },
)
