import {
  createModuleResolver,
  type ModuleResolution,
} from '@devup-ui/plugin-utils'
import { expect, it } from 'bun:test'

import { NonphysicalModules, observeResolver } from '../nonphysical-modules'
import { productionFixture } from './production-fixture'

it.each([false, true])(
  'preserves owned resolver identity and ready-source admission when ignored=%s',
  async (ignored) => {
    // Given real files, a native false alias and a public prepared-source callback.
    const fixture = await productionFixture()
    const importer = await fixture.file('app/src/main.js', 'export {}')
    const leaf = await fixture.file('app/src/leaf.js', 'export const value=1')
    const code = 'export const value=2'
    let preparations = 0
    const resolver = createModuleResolver({
      cwd: fixture.root,
      alias: { ignored: false },
      prepareSource: () => {
        preparations += 1
        return { code, ignored: false }
      },
      toId: (path) => path.replaceAll('\\', '/'),
    })
    const modules = new NonphysicalModules([])
    const root = {
      nativeId: importer,
      id: importer,
      source: { code: 'export {}' },
    }
    modules.direct(root.nativeId, root.id, root.source)
    let produced: ModuleResolution | undefined
    const observed = observeResolver(
      modules,
      root,
      Object.assign(
        (specifier: string, from: string) => {
          produced = resolver(specifier, from)
          return produced
        },
        { remapError: resolver.remapError },
      ),
    )
    try {
      // When the observer resolves through the actual shared factory.
      const result = observed(ignored ? 'ignored' : './leaf.js', importer)
      // Then observation preserves the exact callback object and only admits real source.
      expect(result).toBe(produced)
      expect(result).toEqual(ignored ? { ignored: true } : { path: leaf, code })
      expect(modules.source(leaf)).toEqual(ignored ? undefined : { code })
      expect(preparations).toBe(ignored ? 0 : 1)
    } finally {
      await fixture.close()
    }
  },
)
