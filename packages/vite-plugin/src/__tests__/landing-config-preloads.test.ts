import { resolve } from 'node:path'

import { expect, it } from 'bun:test'
import { type Plugin, resolveConfig } from 'vite'

import {
  type AssetsManifest,
  getPluginApi,
  vitePluginRscMinimal,
} from '../../../../apps/landing/node_modules/@vitejs/plugin-rsc/dist/plugin.js'
import { productionFixture } from './production-fixture'

it.each([true, false])(
  'removes dead proxy preloads from actual landing plugin with entry group=%s',
  async (withEntry) => {
    // Given actual config plugins and typed entry/reference asset dependencies.
    const root = resolve(import.meta.dir, '../../../../apps/landing')
    const config = await resolveConfig(
      { root, configLoader: 'bundle' },
      'build',
    )
    const apiPlugin = vitePluginRscMinimal()
    const cleanup = config.plugins.find(
      (p) => p.name === 'landing:omit-devup-css-proxy-preloads',
    )
    if (!cleanup) throw new TypeError('Expected actual landing cleanup')
    let api: ReturnType<typeof getPluginApi>
    const entry = {
      js: ['/assets/devup-ui.css-DEAD.js', '/assets/main.js'],
      css: ['/assets/devup-ui.css'],
    }
    const reference = {
      js: [
        '/assets/devup-ui-7.css-DEAD.js',
        '/assets/card.js',
        '/assets/vendor-devup-ui.css-LIVE.js',
      ],
      css: ['/assets/card.css'],
    }
    const assets = {
      clientEntryUrl: '/assets/main.js',
      ...(withEntry ? { clientEntryDeps: entry } : {}),
      clientReferenceDeps: { card: reference },
      serverResources: { page: { css: ['/assets/server.css'] } },
    } satisfies AssetsManifest
    const provider: Plugin = {
      name: 'test:provide-rsc-assets',
      configResolved(miniConfig) {
        api = getPluginApi(miniConfig)
      },
      generateBundle: {
        order: 'pre',
        handler() {
          if (!api) throw new TypeError('Expected public MINI build RSC API')
          api.manager.buildAssetsManifest = assets
        },
      },
    }
    const fixture = await productionFixture()
    try {
      const main = await fixture.file(
        'app/src/main.js',
        'export const value=1;',
      )
      // When Vite supplies a genuine client hook context in a tiny normal build.
      const output = await fixture.run(false, [], {
        plugins: [apiPlugin, cleanup, provider],
        build: { write: false, lib: { entry: main, formats: ['es'] } },
      })
      // Then only dead JS preloads disappear; ordinary JS, CSS and metadata remain.
      expect(output.some((item) => item.type === 'chunk')).toBe(true)
      expect(reference.js).toEqual([
        '/assets/card.js',
        '/assets/vendor-devup-ui.css-LIVE.js',
      ])
      expect(reference.css).toEqual(['/assets/card.css'])
      expect(entry.js).toEqual(
        withEntry
          ? ['/assets/main.js']
          : ['/assets/devup-ui.css-DEAD.js', '/assets/main.js'],
      )
      expect(entry.css).toEqual(['/assets/devup-ui.css'])
      expect(assets.clientEntryUrl).toBe('/assets/main.js')
      expect(assets.serverResources).toEqual({
        page: { css: ['/assets/server.css'] },
      })
      expect(api?.manager.buildAssetsManifest).toBe(assets)
    } finally {
      await fixture.close()
    }
  },
  30000,
)
