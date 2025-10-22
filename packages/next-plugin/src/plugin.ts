import { existsSync, mkdirSync, writeFileSync } from 'node:fs'
import { join, relative, resolve } from 'node:path'

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
  const isTurbo =
    process.env.TURBOPACK === '1' || process.env.TURBOPACK === 'auto'
  if (isTurbo) {
    config ??= {}
    config.turbopack ??= {}
    config.turbopack.rules ??= {}
    const {
      package: libPackage = '@devup-ui/react',
      distDir = 'df',
      cssDir = resolve(distDir, 'devup-ui'),
      singleCss = false,
    } = options

    const sheetFile = join(distDir, 'sheet.json')
    const classMapFile = join(distDir, 'classMap.json')
    const fileMapFile = join(distDir, 'fileMap.json')
    const gitignoreFile = join(distDir, '.gitignore')
    if (!existsSync(distDir)) mkdirSync(distDir)
    if (!existsSync(cssDir)) mkdirSync(cssDir)
    if (!existsSync(gitignoreFile)) writeFileSync(gitignoreFile, '*')
    const rules: NonNullable<typeof config.turbopack.rules> = {
      [`./${relative(process.cwd(), cssDir).replaceAll('\\', '/')}/*.css`]: [
        {
          loader: '@devup-ui/next-plugin/css-loader',
          options: {
            watch: process.env.NODE_ENV === 'development',
          },
        },
      ],
      '*.{tsx,ts,js,mjs}': [
        {
          loader: '@devup-ui/next-plugin/loader',
          options: {
            package: libPackage,
            cssDir,
            sheetFile,
            classMapFile,
            fileMapFile,
            watch: process.env.NODE_ENV === 'development',
            singleCss,
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
