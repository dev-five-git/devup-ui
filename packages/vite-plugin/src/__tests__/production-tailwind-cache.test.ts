import * as wasm from '@devup-ui/wasm'
import { expect, it } from 'bun:test'
import { resolveConfig } from 'vite'

import { ProductionActivation } from '../production-numbering'
import { productionFixture } from './production-fixture'

it('clears completed no-op and pending preparation while preserving the current cohort when theme refreshes (UNIT)', async () => {
  // Given two real owner contexts with prior completed no-op preparation.
  const fixture = await productionFixture()
  await fixture.file('app/src/main.js', 'export {};')
  await fixture.file('app/devup.json', '{"theme":{}}')
  const activation = new ProductionActivation(
    {
      package: '@devup-ui/react',
      devupFile: 'devup.json',
      distDir: 'df',
      cssDir: undefined,
      extractCss: true,
      debug: false,
      include: [],
      singleCss: true,
      prefix: undefined,
      shorthands: undefined,
      sourceDirs: undefined,
      mdxExtensions: ['.mdx'],
      atomHoist: undefined,
      importAliases: {},
    },
    () => {},
  )
  const old = 'virtual:old.mdx'
  const current = 'virtual:current.mdx'
  const source = { code: 'export {};', sourceType: 'compiled-mdx' } as const
  const contexts = ['client', 'server'] as const
  try {
    activation.configure(
      await resolveConfig(
        {
          root: fixture.root,
          configFile: false,
          logLevel: 'silent',
          environments: {
            client: { consumer: 'client' },
            server: { consumer: 'server' },
          },
          build: {
            watch: {},
            lib: { entry: `${fixture.root}/src/main.js`, formats: ['es'] },
          },
        },
        'build',
      ),
    )
    await activation.prepare(() => {})
    for (const key of contexts) {
      activation.start(key)
      activation.modules(key).direct(old, old, source, () => false)
      await activation.finish(key, {
        getModuleIds: () => [old].values(),
        getModuleInfo: () => ({
          code: source.code,
          isEntry: true,
          importedIds: [],
          dynamicallyImportedIds: [],
        }),
      })
      activation.start(key)
      const modules = activation.modules(key)
      expect(modules.observations()).toEqual([])
      modules.direct(current, current, source, () => false)
      expect(modules.source(old)).toEqual(source)
      expect(modules.source(current)).toEqual(source)
      expect(modules.observations()).toEqual([current])
    }
    const numbers = activation.run('client', wasm.exportFileMap)
    const stored = await fixture.storeBytes()
    expect(await fixture.learned()).toEqual([])
    await fixture.file(
      'app/devup.json',
      '{"theme":{"colors":{"default":{"brand":"#123456"}}}}',
    )
    // When the real owner refreshes its theme without starting a new generation.
    await activation.updateTheme()
    // Then every preparation cache is empty, but the current cohort and numbers survive.
    for (const key of contexts) {
      const modules = activation.modules(key)
      expect(modules.source(old)).toBeUndefined()
      expect(modules.source(current)).toBeUndefined()
      expect(modules.observations()).toEqual([current])
      expect(activation.run(key, wasm.exportFileMap)).toBe(numbers)
      await activation.finish(key, {
        getModuleIds: () => [current].values(),
        getModuleInfo: () => null,
      })
    }
    expect(await fixture.storeBytes()).toEqual(stored)
    expect(await fixture.learned()).toEqual([])
  } finally {
    activation.dispose()
    await fixture.close()
  }
})
