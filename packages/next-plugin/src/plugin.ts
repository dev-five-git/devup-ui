import {
  DevupUIWebpackPlugin,
  type DevupUIWebpackPluginOptions,
} from '@devup-ui/webpack-plugin'
import { type NextConfig } from 'next'

type DevupUiNextPluginOptions = Partial<DevupUIWebpackPluginOptions>

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
  config.webpack = (config, _options) => {
    config.plugins.push(new DevupUIWebpackPlugin(options))
    if (typeof webpack === 'function') return webpack(config, _options)
    return config
  }
  return config
}
