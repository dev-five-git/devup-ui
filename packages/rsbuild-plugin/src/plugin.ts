import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { mkdir, stat, writeFile } from 'node:fs/promises'
import { basename, dirname, join, relative, resolve } from 'node:path'

import {
  beginBuild,
  buildCanonicalMap,
  buildStaticImportGraph,
  computeFileReach,
  computeReachableFiles,
  conventionalSourceRoots,
  createCompatTypes,
  createModuleResolver,
  createNodeModulesExcludeRegex,
  createStateWriter,
  createThemeInterfaceArgs,
  type CustomShorthands,
  type DevupCompiler,
  type DevupUIBasePluginOptions,
  esmImportConditions,
  getFileNumByFilename,
  type ImportAliases,
  isMissingDirectory,
  loadCompiler,
  loadDevupConfig,
  mergeImportAliases,
  normalizeShorthands,
  planAtomHoist,
  readJsxImportSource,
  setupWarning,
  SOURCE_FILE_REGEX,
  tailwindCssFiles,
  withTailwindCss,
} from '@devup-ui/plugin-utils'
import type { RsbuildPlugin, Rspack, TransformHandler } from '@rsbuild/core'

import { createReservation, staticEntries } from './reservation'

const PLUGIN_NAME = 'devup-ui-rsbuild-plugin'

export interface DevupUIRsbuildPluginOptions extends Pick<
  DevupUIBasePluginOptions,
  'engine'
> {
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
   * Atom-level route-aware hoisting threshold. Defaults to 2; 0 disables it.
   * Positive thresholds are clamped to >= 2. Canonical buckets reached by at
   * least this many routes use global short counter names in devup-ui.css;
   * other buckets keep per-file counter names. Composes with single-importer
   * collapse and serves per-route CSS via getCss(fileNum). Routes are inferred
   * from entry points and dynamic-import targets. Fewer than two routes yields
   * no hoisted plan. Ignored when singleCss is true, including IDs/import flags
   * and shared-chunk configuration.
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
  compiler: DevupCompiler,
  watching: boolean,
): Promise<{ tailwindFiles: string[]; themeError: unknown }> {
  let tailwindFiles: string[] = []
  let themeError: unknown
  try {
    const config = await loadDevupConfig(options.devupFile)
    const theme = config.theme ?? {}

    // Watched even when the theme fails, so fixing the Tailwind CSS reloads it
    tailwindFiles = tailwindCssFiles(config)
    compiler.registerTheme(withTailwindCss(theme, config))
    // Written even when empty, so tokens removed from the theme leave no
    // stale declarations behind.
    await writeFile(
      join(options.distDir, 'theme.d.ts'),
      compiler.getThemeInterface(...createThemeInterfaceArgs(options.package)),
      'utf-8',
    )
  } catch (error) {
    // A one-shot build fails on a theme it cannot load or register: an empty
    // theme would ship unstyled output. A watcher keeps running, since
    // devup.json may be mid-edit, and reports the error until it is fixed.
    if (!watching) throw error
    console.error(error)
    compiler.registerTheme({})
    themeError = error
  }
  // Sequential: writing into cssDir concurrently with its own mkdir loses the
  // race on a cold start (no `df/`) and fails the build with ENOENT.
  if (!existsSync(options.cssDir)) {
    await mkdir(options.cssDir, { recursive: true })
  }
  if (!options.singleCss) {
    await writeFile(
      join(options.cssDir, 'devup-ui.css'),
      compiler.getCss(null, false),
    )
  }
  return { tailwindFiles, themeError }
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
  engine,
  atomHoist = 2,
  importAliases: userImportAliases,
}: Partial<DevupUIRsbuildPluginOptions> = {}): RsbuildPlugin => {
  const normalizedShorthands = normalizeShorthands(shorthands ?? {})
  const excludeRegex = createNodeModulesExcludeRegex(include)
  const compiler = loadCompiler(engine)
  compiler.registerShorthands(normalizedShorthands)
  const importAliases = mergeImportAliases(
    userImportAliases,
    readJsxImportSource(),
  )
  const stateWriter = createStateWriter((path, content, encoding) =>
    encoding ? writeFile(path, content, encoding) : writeFile(path, content),
  )

  return {
    name: PLUGIN_NAME,
    async setup(api) {
      // A build starts from its own options, not from what an earlier build
      // in this process left in the engine
      const endBuild = beginBuild(compiler)
      api.onCloseBuild?.(endBuild)
      compiler.setDebug(debug)
      compiler.setPrefix(prefix ?? null)

      if (!existsSync(distDir)) await mkdir(distDir, { recursive: true })
      await writeFile(join(distDir, '.gitignore'), '*', 'utf-8')
      await writeFile(
        join(distDir, 'compat.d.ts'),
        createCompatTypes(importAliases),
        'utf-8',
      )

      // `rsbuild dev` outlives a broken devup.json; every compilation reports
      // it until it loads again
      let { tailwindFiles, themeError } = await writeDataFiles(
        {
          package: libPackage,
          cssDir,
          devupFile,
          distDir,
          singleCss,
        },
        compiler,
        api.context.action === 'dev',
      )
      if (!extractCss) return

      // The project's Tailwind CSS defines what a Tailwind class compiles to:
      // read it again when it changes, and compile every module again
      api.modifyRspackConfig((config) => {
        config.plugins ??= []
        config.plugins.push({
          apply(owner: {
            hooks: {
              watchRun: {
                tapPromise: (name: string, fn: () => Promise<void>) => void
              }
            }
          }) {
            let stamp: string | undefined
            owner.hooks.watchRun.tapPromise(
              'DevupUIRsbuildPlugin',
              async () => {
                const current = (
                  await Promise.all(
                    tailwindFiles.map((file) =>
                      stat(file).then(
                        (stats) => stats.mtimeMs,
                        () => 0,
                      ),
                    ),
                  )
                ).join()
                if (stamp !== undefined && stamp !== current) {
                  ;({ tailwindFiles, themeError } = await writeDataFiles(
                    {
                      package: libPackage,
                      cssDir,
                      devupFile,
                      distDir,
                      singleCss,
                    },
                    compiler,
                    true,
                  ))
                }
                stamp = current
              },
            )
          },
        })
      })

      // Per-file atom hoisting defaults to 2; 0 disables it, singleCss ignores it.
      // Before any transform, routes are imported, then the threshold is set;
      // bucket eligibility freezes at the first extraction. Late reach cannot promote
      // or rename a bucket until resetBuildState. Composes with
      // single-importer collapse (both keyed by the canonical bucket). rsbuild
      // passes the ABSOLUTE resourcePath to codeExtract, so the graph maps use
      // absolute keys (keyBy: 'absolute') and the extraction filename is
      // POSIX-normalized to match.
      const atomMode = !singleCss && Number.isFinite(atomHoist) && atomHoist > 0
      const moduleResolver = (conditions?: readonly string[]) =>
        createModuleResolver({
          toId: (path) => (atomMode ? path.replaceAll('\\', '/') : path),
          conditions,
        })
      compiler.setModuleResolver(moduleResolver())
      // Each environment resolves imports with its own `exports` conditions
      const environmentResolvers = new Map<
        string,
        ReturnType<typeof moduleResolver>
      >()
      // The `src` and `app` directories of the working directory and of the
      // Rsbuild root, scanned once for the canonical map and the routes
      const sourceGraph = (srcDir: string) =>
        buildStaticImportGraph(
          srcDir,
          resolve(srcDir, 'tsconfig.json'),
          conventionalSourceRoots(process.cwd(), api.context.rootPath),
        )
      if (atomMode) {
        try {
          const root = process.cwd()
          const graph = sourceGraph(root)
          const canonicalMap = buildCanonicalMap({
            srcDir: root,
            cwd: root,
            keyBy: 'absolute',
            graph,
          })
          compiler.importCanonicalMap(canonicalMap)
          const fileReach = computeFileReach({
            srcDir: root,
            cwd: root,
            keyBy: 'absolute',
            graph,
          })
          const plan = planAtomHoist(canonicalMap, fileReach, atomHoist)
          if (plan) {
            compiler.importFileRoutes(plan.reachByBucket)
            compiler.setAtomHoist(plan.threshold)
          } else {
            console.info(
              '[devup-ui] fewer than 2 routes were detected; atom hoisting is a no-op (single-entry/SPA).',
            )
          }
        } catch (error) {
          // Best-effort; on failure atom hoisting stays off (identity).
          if (!isMissingDirectory(error, resolve(process.cwd(), 'src')))
            console.warn(
              setupWarning(
                'Rsbuild import graph',
                error,
                'Atom hoisting is off, so atoms several routes share are not moved into the shared stylesheet.',
              ),
            )
        }
      }

      const reservation = createReservation(
        {
          root: api.context.rootPath,
          package: libPackage,
          include,
          cssDir,
          singleCss,
          atomMode,
          importAliases,
        },
        compiler,
      )
      const entryFiles = new Set<string>()
      // Extract the source files under `src` and `app` that the entries
      // reach, in path order, the same way the transform does, so that a
      // stylesheet built on its first import already holds the styles of every
      // one. Best-effort: a stylesheet still missing styles is rebuilt by
      // another pass.
      const prewarm = () => {
        try {
          const root = api.context.rootPath
          for (const file of computeReachableFiles({
            srcDir: root,
            entries: [...entryFiles],
            graph: sourceGraph(root),
          })) {
            let extractCssDir = relative(dirname(file), cssDir).replaceAll(
              '\\',
              '/',
            )
            if (!extractCssDir.startsWith('./'))
              extractCssDir = `./${extractCssDir}`
            compiler.codeExtract(
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
      }
      api.onBeforeBuild(({ environments }) => {
        for (const { entry } of Object.values(environments)) {
          for (const request of staticEntries(entry))
            entryFiles.add(resolve(api.context.rootPath, request))
        }
        reservation.reserve()
        prewarm()
      })

      const servedCss = new Map<string, Map<string, string>>()
      // Forget deleted source files, under the names the transform extracts
      // them under
      const removeFiles = (files: ReadonlySet<string> | undefined) => {
        for (const file of files ?? [])
          compiler.removeFile(atomMode ? file.replaceAll('\\', '/') : file)
      }
      const stylesheet = (resourcePath: string) =>
        // A file's stylesheet imports the shared base, except in atom mode,
        // where the entry code imports the base itself so that hoisted atoms
        // load once (the injected splitChunks cacheGroup, see
        // modifyRsbuildConfig, emits the base once)
        compiler.getCss(getFileNumByFilename(basename(resourcePath)), !atomMode)

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
      // once more: every module is extracted by then. Keep the consumed CSS
      // across watch revisions so cached stylesheet modules are checked too.
      api.modifyRspackConfig((config, { environment }) => {
        for (const request of staticEntries(environment.entry))
          entryFiles.add(resolve(api.context.rootPath, request))
        environmentResolvers.set(
          environment.name,
          moduleResolver(esmImportConditions(config.resolve)),
        )
        config.plugins ??= []
        config.plugins.push({
          apply(compiler: Rspack.Compiler) {
            reservation.trackCompiler(environment.name, compiler)
            const served = new Map<string, string>()
            servedCss.set(environment.name, served)
            let passes = 0
            compiler.hooks.run.tap(PLUGIN_NAME, () => {
              passes = 0
              reservation.reserve()
            })
            compiler.hooks.invalid.tap(PLUGIN_NAME, reservation.invalidate)
            compiler.hooks.watchRun.tap(PLUGIN_NAME, () => {
              passes = 0
              // A deleted source file is never transformed again; the
              // stylesheets it was in are checked against the sheet below
              removeFiles(compiler.removedFiles)
              reservation.reserve()
            })
            compiler.hooks.thisCompilation.tap(PLUGIN_NAME, (compilation) => {
              if (themeError !== undefined)
                compilation.errors.push(
                  new compiler.rspack.WebpackError(
                    `[devup-ui] ${themeError instanceof Error ? themeError.message : String(themeError)}`,
                  ),
                )
              const basePath = join(cssDir, 'devup-ui.css')
              const base = stylesheet(basePath)
              writeChanged(basePath, base)
              let stale: string[] = []
              compilation.hooks.finishModules.tap(PLUGIN_NAME, () => {
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
                compiler.modifiedFiles = new Set([
                  ...(compiler.modifiedFiles ?? []),
                  ...stale,
                ])
                return true
              })
            })
          },
        })
      })

      api.modifyRsbuildConfig((config) => {
        const theme = compiler.getDefaultTheme()
        if (theme) {
          config.source ??= {}
          config.source.define = {
            'process.env.DEVUP_UI_DEFAULT_THEME': JSON.stringify(
              compiler.getDefaultTheme(),
            ),
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
            // No splitChunks config yet still gets the group (rspack fills
            // in its defaults around it); `false` turns chunk splitting off
            // on purpose, so the base CSS stays in each entry.
            const sc = (rspackConfig.optimization.splitChunks ??= {})
            if (sc) {
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

      const extractModule: TransformHandler = async ({
        code,
        resourcePath,
        addDependency,
        environment,
      }) => {
        if (excludeRegex.test(resourcePath)) return code
        for (const file of tailwindFiles) addDependency(file)
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
        const environmentResolver = environmentResolvers.get(environment.name)
        if (environmentResolver) compiler.setModuleResolver(environmentResolver)
        const {
          code: retCode,
          map,
          cssFile,
          updatedBaseStyle,
          dependencies = [],
        } = compiler.codeExtract(
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
            stateWriter.write(
              join(cssDir, 'devup-ui.css'),
              compiler.getCss(null, false),
              'utf-8',
            ),
          )
        }

        if (cssFile) {
          promises.push(
            stateWriter.write(
              join(cssDir, basename(cssFile)),
              stylesheet(cssFile),
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
      api.transform({ test: SOURCE_FILE_REGEX }, extractModule)
      // MDX compiles to JavaScript in a normal-phase loader such as
      // `@mdx-js/loader`: a `post` transform extracts what it compiled.
      api.transform({ test: /\.mdx$/i, order: 'post' }, extractModule)
    },
  }
}
