import {
  existsSync,
  mkdirSync,
  readFileSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs'
import { join, relative, resolve } from 'node:path'

import {
  buildCanonicalMap,
  computeFileRoutes,
  createNodeModulesExcludeRegex,
  createThemeInterfaceArgs,
  loadDevupConfigSync,
  mergeImportAliases,
  planAtomHoist,
} from '@devup-ui/plugin-utils'
import {
  exportClassMap,
  exportFileMap,
  exportSheet,
  getCss,
  getDefaultTheme,
  getThemeInterface,
  importCanonicalMap,
  importClassMap,
  importFileMap,
  importFileRoutes,
  importSheet,
  registerTheme,
  setAtomHoist,
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
      atomHoist,
      importAliases: userImportAliases,
    } = options

    if (prefix) {
      setPrefix(prefix)
    }

    const importAliases = mergeImportAliases(userImportAliases)

    const sheetFile = join(distDir, 'sheet.json')
    const classMapFile = join(distDir, 'classMap.json')
    const fileMapFile = join(distDir, 'fileMap.json')
    const canonicalMapFile = join(distDir, 'canonicalMap.json')
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
      ...createThemeInterfaceArgs(libPackage),
    )
    if (themeInterface) {
      writeFileSync(join(distDir, 'theme.d.ts'), themeInterface)
    }
    // disable turbo parallel
    const excludeRegex = createNodeModulesExcludeRegex(include, '.mdx.[tj]sx?$')

    const coordinatorPortFile = join(distDir, 'coordinator.port')

    // Pre-pass: single-importer collapse ALWAYS runs (files with exactly one
    // importer merge into that importer's bucket, so their identical atoms share
    // one class). Atom-level hoisting COMPOSES on top: an atom reached by
    // >= atomHoist distinct routes is emitted once into the shared devup-ui.css.
    //
    // The two compose because both are keyed by the canonical bucket: the engine
    // keys property buckets by canonical(filename), and the route-reach map below
    // is folded onto the SAME canonical bucket — so route_count_for_files() looks
    // atoms up by bucket and the lookup hits. `atomHoist` must be configured
    // BEFORE any extraction so atoms receive global (shared) class names; the
    // coordinator shares this WASM instance, so it applies to every /extract.
    const atomMode =
      atomHoist !== undefined && Number.isFinite(atomHoist) && atomHoist > 0
    try {
      const srcDir = resolve(process.cwd(), 'src')
      const tsconfigPath = resolve(process.cwd(), 'tsconfig.json')
      const cwd = process.cwd()
      // Atom hoisting owns the shared-chunk decision, so collapse runs WITHOUT
      // the file-level @global hoist (DEVUP_HOIST_V) in atom mode.
      const hoistV = atomMode
        ? undefined
        : process.env.DEVUP_HOIST_V
          ? Number(process.env.DEVUP_HOIST_V)
          : undefined
      const canonicalMap = buildCanonicalMap({
        srcDir,
        tsconfigPath,
        cwd,
        hoistV,
      })
      importCanonicalMap(canonicalMap)
      writeFileSync(canonicalMapFile, JSON.stringify(canonicalMap))

      if (atomMode) {
        // Fold per-file route reach onto the canonical bucket so the keys match
        // the engine's property bucket keys (canonical(filename)).
        const fileRoutes = computeFileRoutes({ srcDir, tsconfigPath, cwd })
        const plan = planAtomHoist(canonicalMap, fileRoutes, atomHoist)
        if (plan) {
          importFileRoutes(plan.reachByBucket)
          setAtomHoist(plan.threshold)
        } else {
          console.info(
            '[devup-ui] atomHoist is set but fewer than 2 routes were detected; atom hoisting is a no-op.',
          )
        }
      }
    } catch {
      // Pre-pass is best-effort; on failure canonical() is the identity (no
      // merge) and atom hoisting stays off.
    }

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
