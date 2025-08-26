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
    config ??= {}
    config.turbopack ??= {}
    config.turbopack.rules ??= {}
    const {
      package: libPackage = '@devup-ui/react',
      interfacePath = 'df',
      cssDir = resolve(interfacePath, 'devup-ui'),
      splitCss = true,
    } = options

    const sheetFile = join(interfacePath, 'sheet.json')
    const classMapFile = join(interfacePath, 'classMap.json')
    const gitignoreFile = join(interfacePath, '.gitignore')
    if (!existsSync(interfacePath)) mkdirSync(interfacePath)
    if (!existsSync(cssDir)) mkdirSync(cssDir)
    if (!existsSync(gitignoreFile)) writeFileSync(gitignoreFile, '*')
    const rules: NonNullable<typeof config.turbopack.rules> = {
      [basename(cssDir)]: [
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
            cssDir,
            sheetFile,
            classMapFile,
            watch: process.env.NODE_ENV === 'development',
            splitCss,
          },
        },
      ],
    }
    Object.assign(config.turbopack.rules, rules)
    return config
  }

  const { webpack } = config
  config.webpack = (config, _options) => {
    options.cssDir ??= resolve(
      _options.dev ? (options.interfacePath ?? 'df') : '.next/cache',
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
