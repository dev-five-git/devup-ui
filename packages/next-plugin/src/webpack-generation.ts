import type { WebpackGenerationBinding } from '@devup-ui/webpack-plugin'

import { loadWebpackPlugin } from './wasm'

/**
 * @internal Next16.3.6 passes one public context.config through the three
 * configurations (webpack-build/impl.js146-197; hot-reloader-webpack.js556-595).
 * loadConfig caches that identity, so only a LIVE owner may be reused.
 */
export function createWebpackGenerationThread() {
  const generations = new WeakMap<object, WebpackGenerationBinding['owner']>()
  return (context: {
    readonly config: object
    readonly dev: boolean
    readonly isServer: boolean
  }): WebpackGenerationBinding => {
    let owner = generations.get(context.config)
    if (!owner || owner.disposed) {
      owner = loadWebpackPlugin().createWebpackGeneration()
      generations.set(context.config, owner)
    }
    return { owner, complete: context.dev || !context.isServer }
  }
}
