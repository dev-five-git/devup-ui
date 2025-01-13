import { existsSync, readFileSync } from 'node:fs'

import {
  DevupUIWebpackPlugin,
  DevupUiWebpackPluginOptions,
} from '@devup-ui/webpack-plugin'
import { type NextConfig } from 'next'

type DevupUiNextPluginOptions = Partial<DevupUiWebpackPluginOptions>

/**
 * Devup UI Next Plugin
 * @param config
 * @param options
 * @constructor
 */
export function DevupUI(
  config: NextConfig,
  options: DevupUiNextPluginOptions = {},
): NextConfig {
  const { webpack } = config
  if (!options.devupTheme && existsSync('devup.json')) {
    try {
      options.devupTheme = JSON.parse(readFileSync('devup.json', 'utf-8'))?.[
        'theme'
      ]
    } catch (error) {
      console.error(error)
    }
  }

  config.webpack = (config, _options) => {
    config.plugins.push(new DevupUIWebpackPlugin(options))
    if (typeof webpack === 'function') return webpack(config, _options)
    return config
  }
  return config
}
