import { readFile } from 'node:fs/promises'

import { createNonphysicalIdStore } from '@devup-ui/plugin-utils'
import { expect, it, spyOn } from 'bun:test'
import { createBuilder } from 'vite'

import { parseNonphysicalIdList } from '../../../plugin-utils/src/nonphysical-id-list'
import { DevupUI } from '../plugin'
import { productionFixture } from './production-fixture'

it('preserves the exact parser fault when persisted native IDs cannot be parsed', async () => {
  // Given a real store and a narrowly unavailable parser API for its exact bytes.
  const fixture = await productionFixture()
  const store = createNonphysicalIdStore({
    integration: 'Vite',
    resolvedRoot: fixture.root,
    contextKey: 'parser-boundary',
  })
  const ids = ['\0native-parser-boundary.js']
  await store.rewrite(ids, [])
  const serialized = await readFile(store.filePath, 'utf8')
  const cause = new RangeError('native ID parser unavailable')
  const original = JSON.parse
  const parser = spyOn(JSON, 'parse').mockImplementation(
    new Proxy(original, {
      apply(target, receiver, args) {
        if (args[0] === serialized) throw cause
        return Reflect.apply(target, receiver, args)
      },
    }),
  )
  try {
    // When parsing actual persisted bytes through the public parser API.
    let caught: unknown
    try {
      parseNonphysicalIdList(serialized)
    } catch (error) {
      caught = error
    }
    // Then a non-syntax failure is neither converted nor replaced.
    expect(caught).toBe(cause)
  } finally {
    parser.mockRestore()
    await fixture.close()
  }
})

it('uses the SSR condition fallback after a public resolved-config callback removes conditions', async () => {
  // Given a real SSR build and an earlier public configResolved callback.
  const fixture = await productionFixture()
  const entry = await fixture.file('app/src/main.js', 'export const value=42')
  let observed = false
  try {
    // When Vite resolves and builds with the optional legacy SSR conditions absent.
    const output = await fixture.run(false, [], {
      build: { ssr: entry, write: false },
      plugins: [
        {
          name: 'remove-optional-ssr-conditions',
          enforce: 'pre',
          configResolved(config) {
            if (config.ssr.resolve) delete config.ssr.resolve.conditions
            observed = config.ssr.resolve?.conditions === undefined
          },
        },
        DevupUI(),
      ],
    })
    // Then the genuine native SSR output survives the retained fallback.
    expect(observed).toBe(true)
    expect(
      output
        .filter((item) => item.type === 'chunk')
        .map((item) => item.code)
        .join(''),
    ).toContain('42')
  } finally {
    await fixture.close()
  }
}, 30000)

it('rejects an empty native plan after a public buildApp callback removes environments', async () => {
  // Given actual resolved builder environments, not a partial ResolvedConfig.
  const fixture = await productionFixture()
  const entry = await fixture.file('app/src/main.js', 'export const value=42')
  let removed = 0
  try {
    const builder = await createBuilder({
      root: fixture.root,
      configFile: false,
      logLevel: 'silent',
      build: { write: false, lib: { entry, formats: ['es'] } },
      plugins: [
        {
          name: 'remove-public-builder-environments',
          enforce: 'pre',
          buildApp: {
            order: 'pre',
            async handler(builder) {
              for (const key of Object.keys(builder.environments)) {
                delete builder.environments[key]
                removed++
              }
            },
          },
        },
        DevupUI(),
      ],
    })
    // When the actual builder dispatches both public buildApp hooks in order.
    const result = builder.buildApp()
    // Then the original empty-context guard rejects rather than publishing a plan.
    await expect(result).rejects.toMatchObject({
      name: 'ProductionNumberingError',
      reason: 'invalid-plan',
    })
    expect(removed).toBeGreaterThan(0)
  } finally {
    await fixture.close()
  }
}, 30000)
