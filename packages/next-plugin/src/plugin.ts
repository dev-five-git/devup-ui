import { existsSync, mkdirSync, writeFileSync } from 'node:fs'
import { basename, join, resolve } from 'node:path'

import {
  DevupUIWebpackPlugin,
  type DevupUIWebpackPluginOptions,
} from '@devup-ui/webpack-plugin'
import { type NextConfig } from 'next'

type DevupUiNextPluginOptions = Omit<
  Partial<DevupUIWebpackPluginOptions>,
  'watch'
>

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
  const isTurbo = process.env.TURBOPACK === '1'
  if (isTurbo) {
    config.experimental ??= {}
    config.experimental.turbo ??= {}
    config.experimental.turbo.rules ??= {}
    const {
      package: libPackage = '@devup-ui/react',
      interfacePath = '.df',
      cssFile = resolve(interfacePath, 'devup-ui.css'),
    } = options

    const sheetFile = join(interfacePath, 'sheet.json')
    const classMapFile = join(interfacePath, 'classMap.json')
    if (!existsSync(interfacePath)) mkdirSync(interfacePath)
    if (!existsSync(cssFile)) writeFileSync(cssFile, '/* devup-ui */')
    const rules: NonNullable<typeof config.experimental.turbo.rules> = {
      [basename(cssFile)]: [
        {
          loader: '@devup-ui/webpack-plugin/css-loader',
          options: {
            watch: process.env.NODE_ENV === 'development',
          },
        },
      ],
      '*.{tsx,ts,js,mjs}': [
        {
          loader: '@devup-ui/webpack-plugin/loader',
          options: {
            package: libPackage,
            cssFile: cssFile,
            sheetFile,
            classMapFile,
            watch: process.env.NODE_ENV === 'development',
          },
        },
      ],
    }
    Object.assign(config.experimental.turbo.rules, rules)
    return config
  }

  const { webpack } = config
  config.webpack = (config, _options) => {
    options.cssFile ??= resolve(
      options.interfacePath ?? '.next/cache',
      `devup-ui_${_options.buildId}.css`,
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
