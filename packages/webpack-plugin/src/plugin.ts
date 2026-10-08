import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { stat, writeFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { dirname, extname, join, relative, resolve } from 'node:path'

import {
  buildCanonicalMap,
  buildStaticImportGraph,
  collectNumberedFiles,
  computeFileReach,
  computeReachableFiles,
  createCompatTypes,
  createModuleResolver,
  createNodeModulesExcludeRegex,
  createThemeInterfaceArgs,
  type CustomShorthands,
  extractedNeedles,
  getFileNumByFilename,
  type ImportAliases,
  isSelectedSource,
  loadDevupConfigSync,
  mdxSourceFilter,
  mergeImportAliases,
  normalizeMdxExtensions,
  planAtomHoist,
  type ResolutionInputObserver,
  resolutionWatchPath,
  resolveProjectPaths,
  resolveSourceDirs,
  seedFileNumbers,
  SOURCE_FILE_RE,
  type StaticImportGraph,
  type WasmImportAliases,
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
  seedFileMap,
  setAtomHoist,
  setDebug,
  setModuleResolver,
  setPrefix,
} from '@devup-ui/wasm'
import { type Compiler } from 'webpack'

import {
  bindCompilerScope,
  type WebpackBuildScope,
  type WebpackGenerationBinding,
} from './build-scope'
import { registerCompiledGuard } from './compiled-guard'
import { servedCss } from './served-css'

export interface DevupUIWebpackPluginOptions {
  package: string
  cssDir: string
  devupFile: string
  distDir: string
  watch: boolean
  debug: boolean
  include: string[]
  singleCss: boolean
  prefix?: string
  shorthands?: CustomShorthands
  sourceDirs?: string | string[]
  mdxExtensions?: readonly string[]
  /**
   * Atom-level route-aware hoisting threshold.
   *
   * When set, a style atom whose content is reached by `>= atomHoist` distinct
   * entries/routes is emitted once into the shared `devup-ui.css`; route-private
   * atoms stay in their per-route chunk. Clamped to a minimum of 2 (an atom
   * shared by `>= 2` routes is the smallest case worth hoisting). Omit to
   * disable atom hoisting (identity behavior).
   *
   * Composes with single-importer collapse: files used by exactly one importer
   * still merge into that importer's bucket (deduplicating their identical
   * atoms), and atom hoisting then shares atoms across the remaining buckets.
   *
   * Currently honored by the Next.js plugin; other bundlers wire it
   * progressively. No effect where unsupported.
   */
  atomHoist?: number
  /**
   * Import aliases for redirecting imports from other CSS-in-JS libraries
   * Merged with defaults: @emotion/styled, styled-components, @vanilla-extract/css
   * Set to `false` to disable specific aliases
   */
  importAliases?: ImportAliases
}

export class DevupUIWebpackPlugin {
  options: Omit<DevupUIWebpackPluginOptions, 'importAliases'>
  sheetFile: string
  classMapFile: string
  fileMapFile: string
  private importAliases: WasmImportAliases
  private excludeModules: RegExp
  private readonly mdxExtensions: readonly string[]
  private seedWarningEmitted = false
  private pathOptions: {
    readonly devupFile: string
    readonly distDir: string
    readonly cssDir: string
  }

  constructor(
    {
      package: libPackage = '@devup-ui/react',
      devupFile = 'devup.json',
      distDir = 'df',
      cssDir = join(distDir, 'devup-ui'),
      watch = false,
      debug = false,
      include = [],
      singleCss = false,
      prefix,
      shorthands,
      sourceDirs,
      atomHoist,
      importAliases: userImportAliases,
      mdxExtensions,
    }: Partial<DevupUIWebpackPluginOptions> = {},
    private readonly generationBinding?: WebpackGenerationBinding,
  ) {
    this.mdxExtensions = normalizeMdxExtensions(mdxExtensions)
    this.importAliases = mergeImportAliases(userImportAliases)
    this.excludeModules = createNodeModulesExcludeRegex(include)
    this.pathOptions = { devupFile, distDir, cssDir }

    this.options = {
      package: libPackage,
      cssDir,
      devupFile,
      distDir,
      watch,
      debug,
      include,
      singleCss,
      prefix,
      shorthands,
      atomHoist,
      sourceDirs,
    }

    this.sheetFile = join(this.options.distDir, 'sheet.json')
    this.classMapFile = join(this.options.distDir, 'classMap.json')
    this.fileMapFile = join(this.options.distDir, 'fileMap.json')
  }

  writeDataFiles(options = this.options) {
    const config = loadDevupConfigSync(options.devupFile)
    const theme = config.theme ?? {}

    registerTheme(theme)
    const interfaceCode = getThemeInterface(
      ...createThemeInterfaceArgs(options.package),
    )

    writeFileSync(join(options.distDir, 'theme.d.ts'), interfaceCode, {
      encoding: 'utf-8',
    })
    if (!existsSync(options.cssDir))
      mkdirSync(options.cssDir, { recursive: true })
    if (options.watch)
      writeFileSync(join(options.cssDir, 'devup-ui.css'), getCss(null, false))
    return theme
  }

  private prewarmExtractor(options: {
    graph: StaticImportGraph
    entries: string[]
    cwd: string
  }) {
    const { graph, entries, cwd } = options
    for (const file of computeReachableFiles({
      srcDir: graph.files,
      tsconfigPath: resolve(cwd, 'tsconfig.json'),
      entries,
      graph,
    })) {
      if (!SOURCE_FILE_RE.test(file)) continue
      try {
        const relativePath = relative(cwd, file).replaceAll('\\', '/')
        let relCssDir = relative(dirname(file), this.options.cssDir).replaceAll(
          '\\',
          '/',
        )
        if (!relCssDir.startsWith('./')) relCssDir = `./${relCssDir}`
        codeExtract(
          relativePath,
          readFileSync(file, 'utf-8'),
          this.options.package,
          relCssDir,
          this.options.singleCss,
          false,
          true,
          this.importAliases,
        )
      } catch (cause) {
        throw new Error(
          `[devup-ui] prewarm failed at ${file} (root ${cwd}): ${cause instanceof Error ? cause.message : String(cause)}`,
          { cause },
        )
      }
    }
  }

  apply(compiler: Compiler) {
    if (compiler.hooks.afterEnvironment) {
      compiler.hooks.afterEnvironment.tap('DevupUIWebpackPlugin', () => {
        this.setupCompiler(compiler)
      })
    } else {
      this.setupCompiler(compiler)
    }
  }

  private setupCompiler(compiler: Compiler) {
    const scope = bindCompilerScope(compiler, this.generationBinding, {
      integration: 'Webpack',
      root: compiler.context ?? compiler.options.context ?? process.cwd(),
    })
    compiler.hooks.beforeRun.tapPromise('DevupUIBuildGeneration', async () =>
      scope.start(),
    )
    compiler.hooks.watchRun.tapPromise('DevupUIBuildGeneration', async () =>
      scope.start(),
    )
    compiler.hooks.shutdown?.tap('DevupUIBuildGeneration', () => scope.close())
    compiler.hooks.watchClose?.tap('DevupUIBuildGeneration', () =>
      scope.close(),
    )
    compiler.hooks.failed?.tap('DevupUIBuildGeneration', () => {
      if (!compiler.watchMode) scope.abort()
    })
    try {
      scope.run(() => this.configureCompiler(compiler, scope))
    } catch (cause) {
      scope.abort()
      throw cause
    }
  }

  private configureCompiler(compiler: Compiler, scope: WebpackBuildScope) {
    const cwd = compiler.context ?? compiler.options.context ?? process.cwd()
    const paths = resolveProjectPaths(cwd, this.pathOptions)
    this.options = { ...this.options, ...paths }
    const options = this.options
    this.sheetFile = join(paths.distDir, 'sheet.json')
    this.classMapFile = join(paths.distDir, 'classMap.json')
    this.fileMapFile = join(paths.distDir, 'fileMap.json')
    const sourceDirs = resolveSourceDirs(cwd, this.options.sourceDirs)
    const resolveOptions = compiler.options.resolve
    const inputFiles = new Set<string>()
    const missingInputs = new Set<string>()
    const onResolutionInputs: ResolutionInputObserver = (inputs) => {
      for (const path of inputs.fileDependencies)
        inputFiles.add(
          resolutionWatchPath(path, resolveOptions?.symlinks === false),
        )
      for (const path of inputs.missingDependencies)
        missingInputs.add(
          resolutionWatchPath(path, resolveOptions?.symlinks === false),
        )
    }
    compiler.hooks.afterCompile.tap(
      'DevupUIResolutionInputs',
      (compilation) => {
        for (const path of inputFiles) compilation.fileDependencies.add(path)
        for (const path of missingInputs)
          compilation.missingDependencies.add(path)
      },
    )
    const baseConditions = resolveOptions?.conditionNames ?? [
      'webpack',
      compiler.options.mode === 'development' ? 'development' : 'production',
      'browser',
    ]
    const conditions = (
      resolveOptions?.byDependency?.esm?.conditionNames ?? [
        'import',
        'module',
        '...',
      ]
    ).flatMap((condition) =>
      condition === '...' ? baseConditions : [condition],
    )
    const entry = compiler.options.entry
    const entries =
      typeof entry === 'function'
        ? []
        : Object.values(entry ?? {}).flatMap(({ import: requests = [] }) =>
            requests.flatMap((request) => {
              const resource = request
                .slice(request.lastIndexOf('!') + 1)
                .split('?')[0]
              if (
                !resource ||
                resource.startsWith('\0') ||
                (/^[a-z][a-z\d+.-]*:/i.test(resource) &&
                  !/^[a-z]:[\\/]/i.test(resource))
              )
                return []
              return isSelectedSource(resource, this.mdxExtensions) ||
                !extname(resource)
                ? [resolve(cwd, resource)]
                : []
            }),
          )
    const roots = [
      ...new Set([...sourceDirs, ...entries.map((file) => dirname(file))]),
    ]
    const endBuild = () => scope.abort()
    let moduleResolver: ReturnType<typeof createModuleResolver>
    try {
      moduleResolver = createModuleResolver({
        cwd,
        includeMdx: this.mdxExtensions,
        conditions,
        alias: resolveOptions?.alias,
        onResolutionInputs,
        toId: (path) => relative(cwd, path).replaceAll('\\', '/'),
      })
      setModuleResolver(moduleResolver)
    } catch (cause) {
      endBuild()
      throw new Error(
        `[devup-ui] resolver setup failed at ${cwd}: ${cause instanceof Error ? cause.message : String(cause)}`,
        { cause },
      )
    }
    setDebug(this.options.debug)
    setPrefix(this.options.prefix ?? null)
    registerShorthands(this.options.shorthands ?? {})
    const existsDevup = existsSync(this.options.devupFile)
    // read devup.json
    if (!existsSync(this.options.distDir))
      mkdirSync(this.options.distDir, { recursive: true })
    writeFileSync(join(this.options.distDir, '.gitignore'), '*', 'utf-8')
    writeFileSync(
      join(this.options.distDir, 'compat.d.ts'),
      createCompatTypes(this.importAliases),
      'utf-8',
    )

    let theme: ReturnType<typeof this.writeDataFiles>
    try {
      theme = this.writeDataFiles(options)
    } catch (cause) {
      endBuild()
      throw new Error(
        `[devup-ui] theme setup failed at ${cwd}: ${cause instanceof Error ? cause.message : String(cause)}`,
        { cause },
      )
    }

    // Atom-level hoisting (opt-in via `atomHoist`). Configured BEFORE any loader
    // runs codeExtract (apply() body is synchronous, loaders run during
    // compilation) so atoms receive global (shared) class names. The WASM
    // instance is shared in-process with the loaders. Composes with
    // single-importer collapse: both keyed by the canonical bucket. The webpack
    // loader passes relative(process.cwd(), id) as the extraction filename, so
    // the graph maps use cwd-relative keys (keyBy: 'cwd-relative').
    const atomHoist = this.options.atomHoist
    const atomMode =
      atomHoist !== undefined && Number.isFinite(atomHoist) && atomHoist > 0
    // Single-importer collapse ALWAYS runs: files used by exactly one importer
    // merge into that importer's bucket, deduplicating their identical atoms.
    // The canonical map is built + imported unconditionally; only atom HOISTING
    // composes on top when `atomHoist` is set. Mirrors next-plugin's pre-pass.
    let graph: StaticImportGraph
    let routes: Record<string, number[]> = {}
    let threshold: number | undefined
    try {
      const srcDir = roots
      const tsconfigPath = resolve(cwd, 'tsconfig.json')
      graph = buildStaticImportGraph(roots, tsconfigPath, {
        includeMdx: this.mdxExtensions,
        cwd,
        include: this.options.include,
        conditions,
        alias: resolveOptions?.alias,
        onResolutionInputs,
      })
      const canonicalMap = buildCanonicalMap({
        srcDir,
        tsconfigPath,
        cwd,
        keyBy: 'cwd-relative',
        graph,
      })
      importCanonicalMap(canonicalMap)

      if (atomMode) {
        const fileReach = computeFileReach({
          srcDir,
          tsconfigPath,
          cwd,
          keyBy: 'cwd-relative',
          graph,
          entries: entries.length ? entries : undefined,
        })
        const plan = planAtomHoist(canonicalMap, fileReach, atomHoist)
        if (plan) {
          routes = plan.reachByBucket
          threshold = plan.threshold
          importFileRoutes(plan.reachByBucket)
          setAtomHoist(plan.threshold)
        } else {
          console.info(
            '[devup-ui] atomHoist is set but fewer than 2 routes were detected; atom hoisting is a no-op (single-entry/SPA).',
          )
        }
      }
    } catch (cause) {
      endBuild()
      throw new Error(
        `[devup-ui] graph setup failed at ${cwd}: ${cause instanceof Error ? cause.message : String(cause)}`,
        { cause },
      )
    }

    // Number every file the build can extract in path order, so class
    // prefixes do not depend on which file a worker reaches first. Numbers
    // already handed out (a restored map in watch mode) stay.
    try {
      seedFileNumbers(
        { seedFileMap },
        collectNumberedFiles({
          roots,
          includeMdx: this.mdxExtensions,
          include: this.options.include,
          cwd,
          needles: extractedNeedles(this.options.package, this.importAliases),
          toId: (path) => relative(cwd, path).replaceAll('\\', '/'),
        }),
      )
    } catch (cause) {
      if (!this.seedWarningEmitted) {
        this.seedWarningEmitted = true
        console.warn(
          '[devup-ui] deterministic file seeding failed; class IDs now depend on module arrival order',
          { phase: 'seed', root: cwd, cause },
        )
      }
    }
    // Pre-warm the extractor so the css-loader serves COMPLETE CSS.
    //
    // Webpack builds a stylesheet module ONCE, at its FIRST import: the shared
    // base (global styles of every file) and, under collapse, a bucket's
    // devup-ui-N.css shared by several source files. Files extracted after that
    // would be missing, so the files the entries reach are extracted up front
    // (single shared WASM instance). Re-extraction by the per-file loader is
    // then idempotent (set-based atom dedup). Watch mode rebuilds stylesheets
    // through the files the loaders write instead.
    if (!this.options.watch) {
      try {
        this.prewarmExtractor({ graph, entries, cwd })
      } catch (cause) {
        endBuild()
        throw cause
      }
    }

    if (this.options.watch) {
      let lastModifiedTime: number | null = null
      compiler.hooks.watchRun.tapPromise('DevupUIWebpackPlugin', async () => {
        if (existsDevup) {
          const stats = await stat(this.options.devupFile)

          const modifiedTime = stats.mtimeMs
          if (lastModifiedTime && lastModifiedTime !== modifiedTime)
            scope.run(() => {
              theme = this.writeDataFiles(options)
            })

          lastModifiedTime = modifiedTime
        }
      })
    }
    if (existsDevup)
      compiler.hooks.afterCompile.tap('DevupUIWebpackPlugin', (compilation) => {
        compilation.fileDependencies.add(resolve(options.devupFile))
      })

    const definePlugin = new compiler.webpack.DefinePlugin({
      'process.env.DEVUP_UI_DEFAULT_THEME': JSON.stringify(getDefaultTheme()),
    })
    if (compiler.hooks.afterEnvironment) definePlugin.apply(compiler)
    else compiler.options.plugins.push(definePlugin)
    if (!this.options.watch) {
      // A stylesheet module is built on its first import, which can come before
      // the modules whose styles it holds are extracted. When one was, compile
      // once more: every module is extracted by then. Watch mode rebuilds it
      // through the stylesheet files the loaders write instead.
      let passes = 0
      compiler.hooks.run.tap('DevupUIWebpackPlugin', () => {
        passes = 0
      })
      compiler.hooks.thisCompilation.tap(
        'DevupUIWebpackPlugin',
        (compilation) => {
          const basePath = join(options.cssDir, 'devup-ui.css')
          const base = scope.run(() => getCss(null, true))
          // A file's stylesheet `@import`s the shared base, which the CSS
          // loaders read from disk without passing through this plugin, so it
          // must hold every style extracted so far
          if (!existsSync(basePath) || readFileSync(basePath, 'utf-8') !== base)
            writeFileSync(basePath, base, 'utf-8')
          let stale: string[] = []
          compilation.hooks.finishModules.tap('DevupUIWebpackPlugin', () =>
            scope.run(() => {
              if (compiler.watchMode) return
              const changed = new Set(
                [...servedCss(compilation)]
                  .filter(
                    ([path, css]) =>
                      getCss(getFileNumByFilename(path), true) !== css,
                  )
                  .map(([path]) => path),
              )
              if (getCss(null, true) !== base) changed.add(basePath)
              stale = [...changed]
            }),
          )
          // The next pass writes the build; this one writes none of its files
          compilation.hooks.processAssets.tap(
            {
              name: 'DevupUIWebpackPlugin',
              stage: compiler.webpack.Compilation.PROCESS_ASSETS_STAGE_REPORT,
            },
            () => {
              if (stale.length === 0 || passes > 0) return
              for (const name of Object.keys(compilation.assets))
                compilation.deleteAsset(name)
            },
          )
          compilation.hooks.needAdditionalPass.tap(
            'DevupUIWebpackPlugin',
            () => {
              if (stale.length === 0 || passes > 0) return undefined
              passes += 1
              scope.run(() => {
                for (const path of stale)
                  writeFileSync(
                    path,
                    getCss(getFileNumByFilename(path), true),
                    'utf-8',
                  )
              })
              return true
            },
          )
        },
      )
      compiler.hooks.done.tapPromise('DevupUIWebpackPlugin', async (stats) => {
        if (!stats.hasErrors()) {
          // write css file
          await writeFile(
            join(options.cssDir, 'devup-ui.css'),
            scope.run(() => getCss(null, false)),
            'utf-8',
          )
        } else if (!compiler.watchMode) scope.abort()
      })
    }

    const exclude = this.excludeModules
    const sourceLoader = {
      loader: createRequire(import.meta.url).resolve(
        '@devup-ui/webpack-plugin/loader',
      ),
      options: {
        package: this.options.package,
        cssDir: this.options.cssDir,
        sheetFile: this.sheetFile,
        classMapFile: this.classMapFile,
        fileMapFile: this.fileMapFile,
        watch: this.options.watch,
        singleCss: this.options.singleCss,
        importAliases: this.importAliases,
        rootDir: cwd,
        conditions,
        alias: resolveOptions?.alias,
        mdxExtensions: this.mdxExtensions,
        symlinks: resolveOptions?.symlinks,
      },
    }
    compiler.options.module.rules.push(
      {
        test: SOURCE_FILE_RE,
        exclude,
        enforce: 'pre',
        use: [sourceLoader],
      },
      {
        test: mdxSourceFilter(this.mdxExtensions),
        exclude,
        enforce: 'post',
        use: [sourceLoader],
      },
      {
        test: this.options.cssDir,
        enforce: 'pre',
        use: [
          {
            loader: createRequire(import.meta.url).resolve(
              '@devup-ui/webpack-plugin/css-loader',
            ),
            options: {
              watch: this.options.watch,
            },
          },
        ],
      },
    )
    registerCompiledGuard(compiler, {
      package: this.options.package,
      mdxExtensions: this.mdxExtensions,
      importAliases: this.importAliases,
    })
    scope.setConfiguration(() => {
      setDebug(options.debug)
      setPrefix(options.prefix ?? null)
      registerShorthands(options.shorthands ?? {})
      registerTheme(theme)
      importFileRoutes(routes)
      setAtomHoist(threshold)
      setModuleResolver(moduleResolver)
    })
  }
}
