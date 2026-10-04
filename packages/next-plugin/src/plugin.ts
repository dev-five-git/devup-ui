import { resolve } from 'node:path'

import type { DevupUIBasePluginOptions } from '@devup-ui/plugin-utils'
import type { NextConfig } from 'next'

import {
  reloadSetupModuleForTesting,
  resetSetupHandoffsForTesting,
} from './setup-handoff'
import { setupTurbopack } from './turbo-setup'
import { loadWebpackPlugin } from './wasm'

/** Options accepted by the Next.js integration. */
export type DevupUINextPluginOptions = Partial<DevupUIBasePluginOptions> & {
  /** Share atoms reached by at least this many routes. */
  atomHoist?: number
  /**
   * Extract every source file of the graph before the build, not only the
   * files the compiled routes reach. Needed for sources the graph cannot
   * connect (template imports, MDX); costs CSS and time for dead files and
   * fails the build on a dead file that cannot be extracted.
   */
  prewarmAll?: boolean
}

/** @internal Reproduce Next's isolated config-module reload in unit tests. */
export const reloadTurboSetupModuleForTesting = reloadSetupModuleForTesting

/** @internal Keep the process-wide one-use handoff isolated between tests. */
export const resetTurboSetupCacheForTesting = resetSetupHandoffsForTesting

/**
 * Devup UI Next Plugin
 * @param config
 * @param options
 * @constructor
 */
export function DevupUI(
  config: NextConfig,
  options: DevupUINextPluginOptions = {},
): NextConfig {
  // turbopack is now stable, TURBOPACK is set to auto without any flags
  if (process.env.TURBOPACK === '1' || process.env.TURBOPACK === 'auto') {
    return setupTurbopack(config, options)
  }

  const { webpack } = config
  config.webpack = (config, _options) => {
    const { DevupUIWebpackPlugin } = loadWebpackPlugin()
    options.cssDir ??= resolve(
      _options.dev ? (options.distDir ?? 'df') : '.next/cache',
      `devup-ui_${_options.buildId}`,
    )
    config.plugins.push(
      new DevupUIWebpackPlugin({
        ...options,
        watch: _options.dev,
      }),
    )
    if (typeof webpack === 'function') return webpack(config, _options)
    return config
  }
  return config
}
