import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { mkdir, writeFile } from 'node:fs/promises'
import { basename, dirname, join, relative, resolve } from 'node:path'

import {
  beginBuild,
  buildCanonicalMap,
  buildStaticImportGraph,
  collectNumberedFiles,
  computeFileReach,
  computeReachableFiles,
  createCompatTypes,
  createModuleResolver,
  createNodeModulesExcludeRegex,
  createStateWriter,
  createThemeInterfaceArgs,
  type CustomShorthands,
  extractedNeedles,
  getFileNumByFilename,
  GRAPH_SOURCE_FILE_RE,
  type ImportAliases,
  loadDevupConfig,
  mergeImportAliases,
  planAtomHoist,
  remapMdxError,
  resolveProjectPaths,
  resolveSourceDirs,
  seedFileNumbers,
  SOURCE_FILE_RE,
  type StaticImportGraph,
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
  resetBuildState,
  seedFileMap,
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
  sourceDirs?: string | string[]
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
  const config = await loadDevupConfig(options.devupFile)
  const theme = config.theme ?? {}

  registerTheme(theme)
  const interfaceCode = getThemeInterface(
    ...createThemeInterfaceArgs(options.package),
  )

  await writeFile(join(options.distDir, 'theme.d.ts'), interfaceCode, 'utf-8')
  if (!existsSync(options.cssDir))
    await mkdir(options.cssDir, { recursive: true })
  if (!options.singleCss)
    await writeFile(join(options.cssDir, 'devup-ui.css'), getCss(null, false))
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
  cssDir: configuredCssDir,
  devupFile = 'devup.json',
  debug = false,
  singleCss = false,
  prefix,
  shorthands,
  sourceDirs: configuredSourceDirs,
  atomHoist,
  importAliases: userImportAliases,
}: Partial<DevupUIRsbuildPluginOptions> = {}): RsbuildPlugin => {
  registerShorthands(shorthands ?? {})
  const importAliases = mergeImportAliases(userImportAliases)
  const excludeModules = createNodeModulesExcludeRegex(include)
  let seedWarningEmitted = false
  const stateWriter = createStateWriter((path, content, encoding) =>
    encoding ? writeFile(path, content, encoding) : writeFile(path, content),
  )

  return {
    name: PLUGIN_NAME,
    async setup(api) {
      const root = api.context.rootPath
      const {
        cssDir,
        distDir: outputDir,
        devupFile: configFile,
      } = resolveProjectPaths(root, {
        distDir,
        devupFile,
        cssDir: configuredCssDir,
      })
      // A build starts from its own options, not from what an earlier build
      // in this process left in the engine
      const endBuild = beginBuild({ resetBuildState })
      api.onCloseBuild?.(endBuild)
      setDebug(debug)
      setPrefix(prefix ?? null)

      try {
        if (!existsSync(outputDir)) await mkdir(outputDir, { recursive: true })
        await writeFile(join(outputDir, '.gitignore'), '*', 'utf-8')
        await writeFile(
          join(outputDir, 'compat.d.ts'),
          createCompatTypes(importAliases),
          'utf-8',
        )

        await writeDataFiles({
          package: libPackage,
          cssDir,
          devupFile: configFile,
          distDir: outputDir,
          singleCss,
        })
      } catch (cause) {
        endBuild()
        throw new Error(
          `[devup-ui] setup failed at ${root}: ${cause instanceof Error ? cause.message : String(cause)}`,
          { cause },
        )
      }
      if (!extractCss) return

      // Atom-level hoisting (opt-in via `atomHoist`). Configured BEFORE any
      // transform so atoms receive global (shared) class names. Composes with
      // single-importer collapse (both keyed by the canonical bucket). rsbuild
      // passes the ABSOLUTE resourcePath to codeExtract, so the graph maps use
      // absolute keys (keyBy: 'absolute') and the extraction filename is
      // POSIX-normalized to match.
      const atomMode =
        atomHoist !== undefined && Number.isFinite(atomHoist) && atomHoist > 0
      const toId = (path: string) =>
        atomMode ? path.replaceAll('\\', '/') : path
      const sourceDirs = [
        ...new Set([
          ...resolveSourceDirs(root, configuredSourceDirs),
          ...resolveSourceDirs(root),
        ]),
      ]
      const plans = new Map<
        string,
        {
          graph: StaticImportGraph
          roots: string[]
          entries: string[]
          resolver: ReturnType<typeof createModuleResolver>
        }
      >()
      const prewarm = (plan: NonNullable<ReturnType<typeof plans.get>>) => {
        setModuleResolver(plan.resolver)
        for (const file of computeReachableFiles({
          srcDir: plan.roots,
          tsconfigPath: resolve(root, 'tsconfig.json'),
          entries: plan.entries,
          graph: plan.graph,
        })) {
          if (!SOURCE_FILE_RE.test(file)) continue
          try {
            let extractCssDir = relative(dirname(file), cssDir).replaceAll(
              '\\',
              '/',
            )
            if (!extractCssDir.startsWith('./'))
              extractCssDir = `./${extractCssDir}`
            codeExtract(
              toId(file),
              readFileSync(file, 'utf-8'),
              libPackage,
              extractCssDir,
              singleCss,
              atomMode,
              !atomMode,
              importAliases,
            )
          } catch (cause) {
            throw new Error(
              `[devup-ui] prewarm failed at ${file} (root ${root}): ${cause instanceof Error ? cause.message : String(cause)}`,
              { cause },
            )
          }
        }
      }
      api.onBeforeBuild(() => {
        for (const plan of plans.values()) prewarm(plan)
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
        const initialize = (normalized: Rspack.Compiler['options']) => {
          const baseConditions = normalized.resolve?.conditionNames ?? [
            'webpack',
            normalized.mode === 'development' ? 'development' : 'production',
            environment.config?.output?.target === 'node' ? 'node' : 'browser',
          ]
          const conditions = (
            normalized.resolve?.byDependency?.esm?.conditionNames ?? [
              'import',
              'module',
              '...',
            ]
          ).flatMap((condition) =>
            condition === '...' ? baseConditions : [condition],
          )
          const entry = normalized.entry
          if (typeof entry === 'function')
            throw new Error(
              `[devup-ui] graph setup failed at ${root}: dynamic Rspack entries are not available for prewarm`,
            )
          const rawEntries =
            typeof entry === 'string'
              ? [entry]
              : Array.isArray(entry)
                ? entry
                : Object.values(entry ?? {}).flatMap((value) =>
                    typeof value === 'string'
                      ? [value]
                      : Array.isArray(value)
                        ? value
                        : (value.import ?? []),
                  )
          const entries = rawEntries
            .filter((file) => GRAPH_SOURCE_FILE_RE.test(file))
            .map((file) => resolve(root, file))
          const roots = [
            ...new Set([
              ...sourceDirs,
              ...entries.map((file) => dirname(file)),
            ]),
          ]
          const tsconfigPath = resolve(root, 'tsconfig.json')
          try {
            const resolver = createModuleResolver({
              cwd: root,
              conditions,
              toId,
            })
            setModuleResolver(resolver)
            const graph = buildStaticImportGraph(roots, tsconfigPath, {
              includeMdx: true,
              cwd: root,
              include,
              conditions,
              exclude: [basename(outputDir), basename(cssDir)],
            })
            const plan = { graph, roots, entries, resolver }
            plans.set(environment.name, plan)
            if (atomMode) {
              const canonicalMap = buildCanonicalMap({
                srcDir: roots,
                tsconfigPath,
                cwd: root,
                keyBy: 'absolute',
                graph,
              })
              importCanonicalMap(canonicalMap)
              const reach = computeFileReach({
                srcDir: roots,
                tsconfigPath,
                cwd: root,
                keyBy: 'absolute',
                graph,
                entries: entries.length ? entries : undefined,
              })
              const hoist = planAtomHoist(canonicalMap, reach, atomHoist)
              if (hoist) {
                importFileRoutes(hoist.reachByBucket)
                setAtomHoist(hoist.threshold)
              } else {
                console.info(
                  '[devup-ui] atomHoist is set but fewer than 2 routes were detected; atom hoisting is a no-op (single-entry/SPA).',
                )
              }
            }
            try {
              seedFileNumbers(
                { seedFileMap },
                collectNumberedFiles({
                  roots,
                  includeMdx: true,
                  include,
                  cwd: root,
                  needles: extractedNeedles(libPackage, importAliases),
                  toId,
                }),
              )
            } catch (cause) {
              if (!seedWarningEmitted) {
                seedWarningEmitted = true
                console.warn(
                  '[devup-ui] deterministic file seeding failed; class IDs now depend on module arrival order',
                  { phase: 'seed', root, cause },
                )
              }
            }
            prewarm(plan)
          } catch (cause) {
            endBuild()
            throw new Error(
              `[devup-ui] graph setup failed at ${root}: ${cause instanceof Error ? cause.message : String(cause)}`,
              { cause },
            )
          }
        }
        config.plugins ??= []
        config.plugins.push({
          apply(compiler: Rspack.Compiler) {
            initialize(compiler.options)
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
            const splitChunks = rspackConfig.optimization.splitChunks
            if (splitChunks === false)
              throw new Error(
                `[devup-ui] atomHoist requires splitChunks at ${root}; splitChunks: false disables shared CSS`,
              )
            const sc = splitChunks ?? {}
            rspackConfig.optimization.splitChunks = sc
            sc.cacheGroups ??= {}
            sc.cacheGroups['devupUiShared'] = {
              type: 'css/mini-extract',
              name: 'devup-ui-shared',
              test: /[\\/]devup-ui\.css$/,
              chunks: 'all',
              enforce: true,
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

      const extract: Parameters<typeof api.transform>[1] = async ({
        code,
        resourcePath,
        addDependency,
        environment,
      }) => {
        if (excludeModules.test(resourcePath)) return code
        const plan = plans.get(environment?.name)
        if (plan) setModuleResolver(plan.resolver)
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
        let extractCssDir = relative(dirname(resourcePath), cssDir).replaceAll(
          '\\',
          '/',
        )
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
        } = (() => {
          try {
            return codeExtract(
              extractName,
              code,
              libPackage,
              extractCssDir,
              singleCss,
              atomMode,
              !atomMode,
              importAliases,
            )
          } catch (error) {
            if (/\.mdx$/i.test(resourcePath))
              throw remapMdxError(error, resourcePath)
            throw error
          }
        })()
        for (const dependency of dependencies) addDependency(dependency)
        const promises: Promise<void>[] = []
        if (updatedBaseStyle) {
          // update base style
          promises.push(
            stateWriter.write(
              join(cssDir, 'devup-ui.css'),
              getCss(null, false),
              'utf-8',
            ),
          )
        }

        if (cssFile) {
          promises.push(
            stateWriter.write(
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
      }
      api.transform({ test: SOURCE_FILE_RE }, extract)
      // Rsbuild's order: post installs an enforce: post Rspack loader. Its
      // transform context does not expose the incoming map, so located errors
      // are explicitly labelled as compiled MDX rather than claiming raw lines.
      api.transform({ test: /\.mdx$/i, order: 'post' }, extract)
    },
  }
}
