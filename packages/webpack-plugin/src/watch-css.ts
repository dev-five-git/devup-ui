import { getFileNumByFilename } from '@devup-ui/plugin-utils'
import { getCss } from '@devup-ui/wasm'

import { servedCss } from './served-css'

interface WatchCssCompilation<Module extends object> {
  readonly hooks: {
    readonly finishModules: {
      tapPromise(
        options: { readonly name: string; readonly stage: number },
        callback: (modules: Iterable<Module>) => Promise<void>,
      ): void
    }
  }
  rebuildModule(module: Module, callback: (error?: Error | null) => void): void
}

export function reconcileWatchCss<Module extends object>(
  compilation: WatchCssCompilation<Module>,
): void {
  compilation.hooks.finishModules.tapPromise(
    { name: 'DevupUIWebpackPluginWatchCss', stage: Number.MAX_SAFE_INTEGER },
    async (modules) => {
      const served = servedCss(compilation)
      await Promise.all(
        [...modules].map(async (module) => {
          if (!('resource' in module) || typeof module.resource !== 'string')
            return
          const css = served.get(module.resource)
          if (
            css === undefined ||
            css === getCss(getFileNumByFilename(module.resource), true)
          )
            return
          await new Promise<void>((resolve, reject) => {
            compilation.rebuildModule(module, (error) => {
              if (error) reject(error)
              else resolve()
            })
          })
        }),
      )
    },
  )
}
