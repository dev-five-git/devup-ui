import assert from 'node:assert/strict'

import { expect, it } from 'bun:test'
import { build } from 'vite'

import { DevupUI } from '../plugin'
import { nativeWatcher } from './native-watcher'
import { lifecycleHook } from './plugin-lifecycle-fixture'
import { productionFixture } from './production-fixture'

it('unwraps actual function object and absent hooks from the unchanged public factory', () => {
  // Given genuine factory companions with different public Vite hook shapes.
  const plugins = DevupUI()
  assert(Array.isArray(plugins))
  const main: unknown = plugins[0]
  const mdx: unknown = plugins[2]
  const aggregate: unknown = plugins[3]
  assert(typeof main === 'object' && main !== null)
  assert(typeof mdx === 'object' && mdx !== null)
  assert(typeof aggregate === 'object' && aggregate !== null)
  const wrapped: unknown = Reflect.get(main, 'transform')
  const direct: unknown = Reflect.get(mdx, 'transform')
  const absent: unknown = Reflect.get(aggregate, 'transform')
  assert(typeof wrapped === 'object' && wrapped !== null)
  assert(typeof direct === 'function')
  // When both fixture adapters consume real public factory output.
  const handlers = [wrapped, direct, absent].map(lifecycleHook)
  // Then callable identity survives while an actually absent hook stays absent.
  expect(handlers).toEqual([Reflect.get(wrapped, 'handler'), direct, undefined])
})

it.each([{ formats: ['es'] }, { formats: ['es', 'cjs'] }] as const)(
  'rejects completed native %j output where a watcher is required',
  async ({ formats }) => {
    // Given a genuine configurable Vite build with watch disabled.
    const fixture = await productionFixture()
    const entry = await fixture.file('app/src/main.js', 'export const value=42')
    try {
      const result = await build({
        root: fixture.root,
        configFile: false,
        logLevel: 'silent',
        build: { write: false, lib: { entry, formats: [...formats] } },
      })
      // When the shared watcher adapter receives real completed native output.
      // Then the original guard and exact error remain observable.
      expect(() => nativeWatcher(result)).toThrow('Expected native watcher')
    } finally {
      await fixture.close()
    }
  },
  30000,
)
