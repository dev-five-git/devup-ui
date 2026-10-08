import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { mkdir, writeFile } from 'node:fs/promises'
import { basename, dirname, join, relative, resolve } from 'node:path'

import {
  buildCanonicalMap,
  computeFileReach,
  computeReachableFiles,
  createCompatTypes,
  createModuleResolver,
  createNodeModulesExcludeRegex,
  createThemeInterfaceArgs,
  type CustomShorthands,
  getFileNumByFilename,
  type ImportAliases,
  loadDevupConfig,
  mergeImportAliases,
  planAtomHoist,
  readJsxImportSource,
} from '@devup-ui/plugin-utils'
import {
  codeExtract,
  getCss,
  getDefaultTheme,
  getThemeInterface,
  importCanonicalMap,
  importFileRoutes,
  registerShorthands,
  registerTheme,
  setAtomHoist,
  setDebug,
  setModuleResolver,
  setPrefix,
} from '@devup-ui/wasm'
import type { RsbuildPlugin, Rspack } from '@rsbuild/core'

const PLUGIN_NAME = 'devup-ui-rsbuild-plugin'

export interface DevupUIRsbuildPluginOptions {
  package: string
  cssDir: string
  devupFile: string
  distDir: string
  extractCss: boolean
  debug: boolean
  include: string[]
  singleCss: boolean
  prefix?: string
  shorthands?: CustomShorthands
  /**
   * Atom-level route-aware hoisting threshold (min routes sharing an atom for it
   * to hoist into the shared devup-ui.css; clamped to >= 2; omit to disable).
   * Opt-in: when set, single-importer collapse + atom hoisting are enabled and
   * per-route CSS is served via getCss(fileNum). "Routes" are inferred from the
   * import graph (entry points and dynamic-import targets). For a single-entry
   * SPA (routeCount < 2) it is a no-op.
   *
   * On MPA, the shared base devup-ui.css (hoisted atoms) is emitted as ONE
   * shared chunk via an injected rspack `splitChunks` cacheGroup
   * (`type: 'css/mini-extract'`), so hoisting actually deduplicates across
   * entries rather than being inlined per entry.
   */
  atomHoist?: number
  /**
   * Import aliases for redirecting imports from other CSS-in-JS libraries
   * Merged with defaults: @emotion/styled, styled-components, @vanilla-extract/css
   * Set to `false` to disable specific aliases
   */
  importAliases?: ImportAliases
}

async function writeDataFiles(
  options: Omit<
    DevupUIRsbuildPluginOptions,
    'extractCss' | 'debug' | 'include'
  >,
) {
  try {
    const config = await loadDevupConfig(options.devupFile)
    const theme = config.theme ?? {}

    registerTheme(theme)
    const interfaceCode = getThemeInterface(
      ...createThemeInterfaceArgs(options.package),
    )

    if (interfaceCode) {
      await writeFile(
        join(options.distDir, 'theme.d.ts'),
        interfaceCode,
        'utf-8',
      )
    }
  } catch (error) {
    console.error(error)
    registerTheme({})
  }
  await Promise.all([
    !existsSync(options.cssDir)
      ? mkdir(options.cssDir, { recursive: true })
      : Promise.resolve(),
    !options.singleCss
      ? writeFile(join(options.cssDir, 'devup-ui.css'), getCss(null, false))
      : Promise.resolve(),
  ])
}

/**
 * Write `css` to the stylesheet file at `path` unless it holds it already. The
 * CSS loaders read the shared base from disk when a file's stylesheet imports
 * it, so it must hold every style extracted so far.
 */
function writeChanged(path: string, css: string) {
  if (!existsSync(path) || readFileSync(path, 'utf-8') !== css)
    writeFileSync(path, css, 'utf-8')
}

export const DevupUI = ({
  include = [],
  package: libPackage = '@devup-ui/react',
  extractCss = true,
  distDir = 'df',
  cssDir = resolve(distDir, 'devup-ui'),
  devupFile = 'devup.json',
  debug = false,
  singleCss = false,
  prefix,
  shorthands,
  atomHoist,
  importAliases: userImportAliases,
}: Partial<DevupUIRsbuildPluginOptions> = {}): RsbuildPlugin => {
  registerShorthands(shorthands ?? {})
  const importAliases = mergeImportAliases(
    userImportAliases,
    readJsxImportSource(),
  )

  return {
    name: PLUGIN_NAME,
    async setup(api) {
      setDebug(debug)
      if (prefix) {
        setPrefix(prefix)
      }

      if (!existsSync(distDir)) await mkdir(distDir, { recursive: true })
      await writeFile(join(distDir, '.gitignore'), '*', 'utf-8')
      await writeFile(
        join(distDir, 'compat.d.ts'),
        createCompatTypes(importAliases),
        'utf-8',
      )

      await writeDataFiles({
        package: libPackage,
        cssDir,
        devupFile,
        distDir,
        singleCss,
      })
      if (!extractCss) return

      // Atom-level hoisting (opt-in via `atomHoist`). Configured BEFORE any
      // transform so atoms receive global (shared) class names. Composes with
      // single-importer collapse (both keyed by the canonical bucket). rsbuild
      // passes the ABSOLUTE resourcePath to codeExtract, so the graph maps use
      // absolute keys (keyBy: 'absolute') and the extraction filename is
      // POSIX-normalized to match.
      const atomMode =
        atomHoist !== undefined && Number.isFinite(atomHoist) && atomHoist > 0
      setModuleResolver(
        createModuleResolver({
          toId: (path) => (atomMode ? path.replaceAll('\\', '/') : path),
        }),
      )
      if (atomMode) {
        try {
          const root = process.cwd()
          const srcDir = resolve(root, 'src')
          const tsconfigPath = resolve(root, 'tsconfig.json')
          const canonicalMap = buildCanonicalMap({
            srcDir,
            tsconfigPath,
            cwd: root,
            keyBy: 'absolute',
          })
          importCanonicalMap(canonicalMap)
          const fileReach = computeFileReach({
            srcDir,
            tsconfigPath,
            cwd: root,
            keyBy: 'absolute',
          })
          const plan = planAtomHoist(canonicalMap, fileReach, atomHoist)
          if (plan) {
            importFileRoutes(plan.reachByBucket)
            setAtomHoist(plan.threshold)
          } else {
            console.info(
              '[devup-ui] atomHoist is set but fewer than 2 routes were detected; atom hoisting is a no-op (single-entry/SPA).',
            )
          }
        } catch {
          // Best-effort; on failure atom hoisting stays off (identity).
        }
      }

      // Extract the source files under `src` that the entries reach, in path
      // order, the same way the transform does, so that a stylesheet built on
      // its first import already holds the styles of every one. Best-effort:
      // a stylesheet still missing styles is rebuilt by another pass.
      api.onBeforeBuild(({ environments }) => {
        try {
          const root = api.context.rootPath
          const entries = Object.values(environments).flatMap(({ entry }) =>
            Object.values(entry).flatMap((value) =>
              (typeof value === 'object' && !Array.isArray(value)
                ? [value.import].flat()
                : [value].flat()
              ).map((request) => resolve(root, request)),
            ),
          )
          for (const file of computeReachableFiles({
            srcDir: resolve(root, 'src'),
            tsconfigPath: resolve(root, 'tsconfig.json'),
            entries,
          })) {
            let extractCssDir = relative(dirname(file), cssDir).replaceAll(
              '\\',
              '/',
            )
            if (!extractCssDir.startsWith('./'))
              extractCssDir = `./${extractCssDir}`
            codeExtract(
              atomMode ? file.replaceAll('\\', '/') : file,
              readFileSync(file, 'utf-8'),
              libPackage,
              extractCssDir,
              singleCss,
              atomMode,
              !atomMode,
              importAliases,
            )
          }
        } catch {
          // The transform reports the error of the file it cannot extract
        }
      })

      const servedCss = new Map<string, Map<string, string>>()
      const stylesheet = (resourcePath: string) =>
        // A file's stylesheet imports the shared base, except in atom mode,
        // where the entry code imports the base itself so that hoisted atoms
        // load once (the injected splitChunks cacheGroup, see
        // modifyRsbuildConfig, emits the base once)
        getCss(getFileNumByFilename(basename(resourcePath)), !atomMode)

      api.transform(
        {
          test: cssDir,
        },
        ({ resourcePath, environment }) => {
          const css = stylesheet(resourcePath)
          servedCss.get(environment.name)?.set(resourcePath, css)
          return css
        },
      )

      // A stylesheet module is built on its first import, which can come before
      // the modules whose styles it holds are extracted. When one was, compile
      // once more: every module is extracted by then. The dev server rebuilds it
      // through the stylesheet files the transforms write instead.
      api.modifyRspackConfig((config, { environment }) => {
        config.plugins ??= []
        config.plugins.push({
          apply(compiler: Rspack.Compiler) {
            let passes = 0
            compiler.hooks.run.tap(PLUGIN_NAME, () => {
              passes = 0
            })
            compiler.hooks.thisCompilation.tap(PLUGIN_NAME, (compilation) => {
              const served = new Map<string, string>()
              servedCss.set(environment.name, served)
              const basePath = join(cssDir, 'devup-ui.css')
              const base = stylesheet(basePath)
              writeChanged(basePath, base)
              let stale: string[] = []
              compilation.hooks.finishModules.tap(PLUGIN_NAME, () => {
                if (compiler.watchMode) return
                const changed = new Set(
                  [...served]
                    .filter(([path, css]) => stylesheet(path) !== css)
                    .map(([path]) => path),
                )
                // A file's stylesheet `@import`s the shared base, which the CSS
                // loaders read from disk without passing through this plugin
                if (stylesheet(basePath) !== base) changed.add(basePath)
                stale = [...changed]
              })
              // The next pass writes the build; this one writes none of its files
              compilation.hooks.processAssets.tap(
                {
                  name: PLUGIN_NAME,
                  stage:
                    compiler.rspack.Compilation.PROCESS_ASSETS_STAGE_REPORT,
                },
                () => {
                  if (stale.length === 0 || passes > 0) return
                  for (const name of Object.keys(compilation.assets))
                    compilation.deleteAsset(name)
                },
              )
              compilation.hooks.needAdditionalPass.tap(PLUGIN_NAME, () => {
                if (stale.length === 0 || passes > 0) return false
                passes += 1
                for (const path of stale)
                  writeFileSync(path, stylesheet(path), 'utf-8')
                return true
              })
            })
          },
        })
      })

      api.modifyRsbuildConfig((config) => {
        const theme = getDefaultTheme()
        if (theme) {
          config.source ??= {}
          config.source.define = {
            'process.env.DEVUP_UI_DEFAULT_THEME':
              JSON.stringify(getDefaultTheme()),
            ...config.source.define,
          }
        }
        if (atomMode) {
          // Emit the shared base devup-ui.css (hoisted atoms) as ONE chunk
          // instead of rspack's default per-entry inlining, so hoisting actually
          // deduplicates across MPA entries. Composed (not overwritten) with any
          // user `tools.rspack`.
          config.tools ??= {}
          const prev = config.tools.rspack
          const addSharedCssGroup = (rspackConfig: {
            optimization?: {
              splitChunks?:
                false | { cacheGroups?: Record<string, unknown> } | undefined
            }
          }) => {
            rspackConfig.optimization ??= {}
            const sc = rspackConfig.optimization.splitChunks
            if (sc && typeof sc === 'object') {
              sc.cacheGroups ??= {}
              sc.cacheGroups['devupUiShared'] = {
                type: 'css/mini-extract',
                name: 'devup-ui-shared',
                test: /[\\/]devup-ui\.css$/,
                chunks: 'all',
                enforce: true,
              }
            }
          }
          config.tools.rspack = Array.isArray(prev)
            ? [...prev, addSharedCssGroup]
            : prev != null
              ? [prev, addSharedCssGroup]
              : addSharedCssGroup
        }
        return config
      })

      api.transform(
        {
          test: /\.(tsx|ts|js|mjs|jsx)$/,
        },
        async ({ code, resourcePath, addDependency }) => {
          if (createNodeModulesExcludeRegex(include).test(resourcePath))
            return code
          // The stylesheet import is emitted relative to the importing file, as
          // in the next/webpack/vite loaders. An absolute cssDir would bake this
          // checkout's path into the emitted module, so byte-identical sources
          // in two checkouts (git worktrees, a CI matrix, sibling clones) would
          // produce different output and any content-addressed or relocated
          // build cache would serve the wrong checkout's stylesheet.
          //
          // Atom mode additionally mirrors vite: the entry CODE imports the
          // shared base (import_main_css_in_code=true) so rspack emits
          // devup-ui.css once and links it from every entry (hoisted atoms
          // shared, not inlined), and the extraction filename is
          // POSIX-normalized to match the absolute-keyed canonical map /
          // FILE_ROUTES.
          let extractCssDir = relative(
            dirname(resourcePath),
            cssDir,
          ).replaceAll('\\', '/')
          if (!extractCssDir.startsWith('./'))
            extractCssDir = `./${extractCssDir}`
          const extractName = atomMode
            ? resourcePath.replaceAll('\\', '/')
            : resourcePath
          const {
            code: retCode,
            map,
            cssFile,
            updatedBaseStyle,
            dependencies = [],
          } = codeExtract(
            extractName,
            code,
            libPackage,
            extractCssDir,
            singleCss,
            atomMode,
            !atomMode,
            importAliases,
          )
          for (const dependency of dependencies) addDependency(dependency)
          const promises: Promise<void>[] = []
          if (updatedBaseStyle) {
            // update base style
            promises.push(
              writeFile(
                join(cssDir, 'devup-ui.css'),
                getCss(null, false),
                'utf-8',
              ),
            )
          }

          if (cssFile) {
            promises.push(
              writeFile(
                join(cssDir, basename(cssFile)),
                `/* ${resourcePath} ${Date.now()} */`,
                'utf-8',
              ),
            )
          }
          await Promise.all(promises)
          return {
            code: retCode,
            map,
          }
        },
      )
    },
  }
}
