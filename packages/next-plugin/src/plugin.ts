import {
  DevupUiWebpackPlugin,
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
export function DevupUiNextPlugin(
  config: NextConfig,
  options: DevupUiNextPluginOptions = {},
): NextConfig {
  const { webpack } = config

  config.webpack = (config, _options) => {
    config.plugins.push(new DevupUiWebpackPlugin(options))
    if (typeof webpack === 'function') return webpack(config, _options)
    return config
  }
  return config
}
