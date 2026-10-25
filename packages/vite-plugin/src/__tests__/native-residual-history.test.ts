import { expect, it } from 'bun:test'
import { createBuilder, type InlineConfig } from 'vite'

import { DevupUI } from '../plugin'
import { productionFixture } from './production-fixture'
import {
  assertRetained,
  captureHistory,
  restoreHistory,
} from './reservation-native-history'

it('retains exact multi-context history bytes across cold and learned native builds', async () => {
  // Given real sibling builds that each learn the same opaque extraction ID.
  const fixture = await productionFixture()
  const entry = await fixture.file(
    'app/src/main.js',
    'export {cls} from "virtual-style"',
  )
  const options: InlineConfig = {
    root: fixture.root,
    configFile: false,
    logLevel: 'silent',
    build: { write: false, lib: { entry, formats: ['es'] } },
    environments: { client: {}, sibling: { consumer: 'client' } },
    plugins: [
      DevupUI(),
      {
        name: 'native-multi-history-provider',
        resolveId(id) {
          if (id === 'virtual-style') return '\0virtual-styled.js'
        },
        load(id) {
          if (id === '\0virtual-styled.js')
            return "import {css} from '@devup-ui/react'; export const cls=css({background:'red'});"
        },
      },
    ],
  }
  try {
    const cold = await createBuilder(options)
    await cold.buildApp()
    const before = captureHistory(fixture.root)
    expect(before.kind).toBe('present')
    if (before.kind !== 'present')
      throw new TypeError('Expected native history')
    expect(before.files.length).toBeGreaterThan(1)
    // When a fresh native builder consumes the real learned files in reverse order.
    const learned = await createBuilder({
      ...options,
      builder: {
        async buildApp(builder) {
          for (const environment of Object.values(
            builder.environments,
          ).reverse())
            await builder.build(environment)
        },
      },
    })
    await learned.buildApp()
    const after = captureHistory(fixture.root)
    // Then exact bytes and real opaque IDs survive; restoring them also round-trips.
    assertRetained(before, after)
    expect(after).toEqual(before)
    restoreHistory(fixture.root, before)
    expect(captureHistory(fixture.root)).toEqual(before)
  } finally {
    await fixture.close()
  }
}, 60000)
