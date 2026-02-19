import {
  existsSync,
  mkdirSync,
  readFileSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs'
import { join, relative, resolve } from 'node:path'

import {
  createNodeModulesExcludeRegex,
  loadDevupConfigSync,
  mergeImportAliases,
} from '@devup-ui/plugin-utils'
import {
  exportClassMap,
  exportFileMap,
  exportSheet,
  getCss,
  getDefaultTheme,
  getThemeInterface,
  importClassMap,
  importFileMap,
  importSheet,
  registerTheme,
  setPrefix,
} from '@devup-ui/wasm'
import {
  DevupUIWebpackPlugin,
  type DevupUIWebpackPluginOptions,
} from '@devup-ui/webpack-plugin'
import { type NextConfig } from 'next'

import { startCoordinator } from './coordinator'

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
      prefix,
      importAliases: userImportAliases,
    } = options

    if (prefix) {
      setPrefix(prefix)
    }

    const importAliases = mergeImportAliases(userImportAliases)

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
    // Import previous session state to handle Turbopack persistent cache.
    // When the dev server restarts, Turbopack may skip re-running loaders for
    // unchanged files. Without importing previous state, the coordinator's WASM
    // starts empty and CSS for cached files would be missing.
    try {
      importSheet(JSON.parse(readFileSync(sheetFile, 'utf-8')))
      importClassMap(JSON.parse(readFileSync(classMapFile, 'utf-8')))
      importFileMap(JSON.parse(readFileSync(fileMapFile, 'utf-8')))
    } catch {
      // No previous session state (first run) or corrupt files — start fresh
    }

    const devupConfig = loadDevupConfigSync(devupFile)

    const theme: any = devupConfig.theme ?? {}
    // Register current theme after importing previous state,
    // since importSheet replaces the entire sheet including its theme.
    registerTheme(theme)
    const themeInterface = getThemeInterface(
      libPackage,
      'CustomColors',
      'DevupThemeTypography',
      'DevupTheme',
    )
    if (themeInterface) {
      writeFileSync(join(distDir, 'theme.d.ts'), themeInterface)
    }
    // disable turbo parallel
    const excludeRegex = createNodeModulesExcludeRegex(include, '.mdx.[tj]sx?$')

    const coordinatorPortFile = join(distDir, 'coordinator.port')

    // create devup-ui.css file
    writeFileSync(join(cssDir, 'devup-ui.css'), getCss(null, false))

    // Delete stale port file from previous session so loaders don't connect
    // to a dead coordinator port. The new coordinator writes a fresh port file
    // once it starts listening.
    try {
      unlinkSync(coordinatorPortFile)
    } catch {
      // Port file doesn't exist (first run) — safe to ignore
    }

    const coordinator = startCoordinator({
      package: libPackage,
      cssDir,
      singleCss,
      sheetFile,
      classMapFile,
      fileMapFile,
      importAliases: importAliases as unknown as Record<string, string | null>,
      coordinatorPortFile,
    })

    // Cleanup on exit
    process.on('exit', () => {
      coordinator.close()
    })
    const defaultSheet = JSON.parse(exportSheet())
    const defaultClassMap = JSON.parse(exportClassMap())
    const defaultFileMap = JSON.parse(exportFileMap())
    // for theme script
    const defaultTheme = getDefaultTheme()
    if (defaultTheme) {
      process.env.DEVUP_UI_DEFAULT_THEME = defaultTheme
      config.env ??= {}
      Object.assign(config.env, {
        DEVUP_UI_DEFAULT_THEME: defaultTheme,
      })
    }

    const rules: NonNullable<typeof config.turbopack.rules> = {
      [`./${relative(process.cwd(), cssDir).replaceAll('\\', '/')}/*.css`]: [
        {
          loader: '@devup-ui/next-plugin/css-loader',
          options: {
            watch: process.env.NODE_ENV === 'development',
            coordinatorPortFile,
            sheetFile,
            classMapFile,
            fileMapFile,
            themeFile: devupFile,
            defaultSheet,
            defaultClassMap,
            defaultFileMap,
            theme,
          },
        },
      ],
      '*.{tsx,ts,js,mjs}': {
        loaders: [
          {
            loader: '@devup-ui/next-plugin/loader',
            options: {
              package: libPackage,
              cssDir,
              coordinatorPortFile,
              sheetFile,
              classMapFile,
              fileMapFile,
              themeFile: devupFile,
              defaultSheet,
              defaultClassMap,
              defaultFileMap,
              watch: process.env.NODE_ENV === 'development',
              singleCss,
              // for turbopack, load theme is required on loader
              theme,
              importAliases: importAliases as unknown as Record<string, string>,
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
