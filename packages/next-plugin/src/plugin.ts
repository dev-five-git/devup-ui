import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join, relative, resolve } from 'node:path'

import { exportClassMap, exportFileMap, exportSheet } from '@devup-ui/wasm'
import {
  DevupUIWebpackPlugin,
  type DevupUIWebpackPluginOptions,
} from '@devup-ui/webpack-plugin'
import { type NextConfig } from 'next'

import { preload } from './preload'

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
  // turbopack is now stable, TURBOPACK is set to auto without any flags
  if (isTurbo) {
    // if (process.env.NODE_ENV === 'production') {
    //   throw new Error('Devup UI is not supported in production with turbopack')
    // }

    config ??= {}
    config.turbopack ??= {}
    config.turbopack.rules ??= {}
    const {
      package: libPackage = '@devup-ui/react',
      distDir = 'df',
      cssDir = resolve(distDir, 'devup-ui'),
      singleCss = false,
      devupFile = 'devup.json',
      include = [],
    } = options

    const sheetFile = join(distDir, 'sheet.json')
    const classMapFile = join(distDir, 'classMap.json')
    const fileMapFile = join(distDir, 'fileMap.json')
    const gitignoreFile = join(distDir, '.gitignore')
    if (!existsSync(distDir))
      mkdirSync(distDir, {
        recursive: true,
      })
    if (!existsSync(cssDir))
      mkdirSync(cssDir, {
        recursive: true,
      })
    if (!existsSync(gitignoreFile)) writeFileSync(gitignoreFile, '*')
    const theme = existsSync(devupFile)
      ? JSON.parse(readFileSync(devupFile, 'utf-8'))?.['theme']
      : {}
    // disable turbo parallel
    const excludeRegex = new RegExp(
      `node_modules(?!.*(${['@devup-ui', ...include]
        .join('|')
        .replaceAll('/', '[\\/\\\\_]')})([\\/\\\\.]|$))`,
    )

    if (process.env.NODE_ENV !== 'production') {
      process.env.TURBOPACK_DEBUG_JS = '*'
      process.env.NODE_OPTIONS ??= ''
      process.env.NODE_OPTIONS += ' --inspect-brk'
    } else {
      preload(excludeRegex, libPackage, singleCss, theme, cssDir)
    }

    const rules: NonNullable<typeof config.turbopack.rules> = {
      [`./${relative(process.cwd(), cssDir).replaceAll('\\', '/')}/*.css`]: [
        {
          loader: '@devup-ui/webpack-plugin/css-loader',
        },
      ],
      '*.{tsx,ts,js,mjs}': {
        loaders: [
          {
            loader: '@devup-ui/webpack-plugin/loader',
            options: {
              package: libPackage,
              cssDir,
              sheetFile,
              classMapFile,
              fileMapFile,
              defaultSheet: JSON.parse(exportSheet()),
              defaultClassMap: JSON.parse(exportClassMap()),
              defaultFileMap: JSON.parse(exportFileMap()),
              watch: process.env.NODE_ENV === 'development',
              singleCss,
              // for turbopack, load theme is required on loader
              theme,
            },
          },
        ],
        condition: {
          not: {
            path: excludeRegex,
          },
        },
      },
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
