import { DevupUI } from '@devup-ui/vite-plugin'
import mdx from '@mdx-js/rollup'
import { getPluginApi } from '@vitejs/plugin-rsc/plugin'
import vinext from 'vinext'
import { defineConfig, type Plugin, type ResolvedConfig } from 'vite'

const DEVUP_CSS_PROXY_CHUNK = /\/devup-ui(?:-\d+)?\.css-[^/]+\.js$/u

/**
 * Devup's CSS proxy modules become real CSS assets and intentionally emit no
 * JavaScript. The RSC asset manifest is assembled before Vite removes those
 * empty proxy chunks, so omit their dead modulepreload entries at the source.
 */
function omitDevupCssProxyPreloads(): Plugin {
  let resolvedConfig: ResolvedConfig | undefined

  return {
    name: 'landing:omit-devup-css-proxy-preloads',
    enforce: 'post',
    configResolved(config) {
      resolvedConfig = config
    },
    generateBundle: {
      order: 'post',
      handler() {
        if (this.environment.name !== 'client' || !resolvedConfig) return

        const manifest =
          getPluginApi(resolvedConfig)?.manager.buildAssetsManifest
        if (!manifest) return

        const dependencyGroups = [
          manifest.clientEntryDeps,
          ...Object.values(manifest.clientReferenceDeps),
        ]

        for (const dependencies of dependencyGroups) {
          if (!dependencies) continue
          dependencies.js = dependencies.js.filter(
            (url) =>
              typeof url !== 'string' || !DEVUP_CSS_PROXY_CHUNK.test(url),
          )
        }
      },
    },
  }
}

export default defineConfig({
  define: {
    'process.env.DEVUP_UI_DEFAULT_THEME': JSON.stringify('light'),
  },
  plugins: [
    DevupUI({ singleCss: process.env.DEVUP_SINGLE_CSS === '1' }),
    mdx({ providerImportSource: '@/mdx-components' }),
    vinext(),
    omitDevupCssProxyPreloads(),
  ],
})
