import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { stat, writeFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { dirname, join, relative, resolve } from 'node:path'

import {
  beginBuild,
  buildCanonicalMap,
  collectNumberedFiles,
  computeFileReach,
  computeReachableFiles,
  createCompatTypes,
  createModuleResolver,
  createNodeModulesExcludeRegex,
  createThemeInterfaceArgs,
  type CustomShorthands,
  type DevupCompiler,
  type DevupUIBasePluginOptions,
  esmImportConditions,
  extractedNeedles,
  getFileNumByFilename,
  type ImportAliases,
  loadCompiler,
  loadDevupConfigSync,
  mergeImportAliases,
  normalizeShorthands,
  normalizeStylexOptions,
  planAtomHoist,
  readJsxImportSource,
  reservationGraph,
  type ReservationInventory,
  reservationRoutes,
  resolveDevupFile,
  resolveProjectPath,
  seedFileNumbers,
  setupWarning,
  SOURCE_FILE_REGEX,
  type StaticImportGraph,
  type StylexStyleResolution,
  tailwindCssFiles,
  type WasmImportAliases,
  withTailwindCss,
} from '@devup-ui/plugin-utils'
import { type Compiler, type RuleSetRule } from 'webpack'

import { createWebpackReservation } from './reservation'
import { servedCss } from './served-css'
import { reconcileWatchCss } from './watch-css'

/** The graph of the files a reservation discovered */
interface DiscoveredGraph {
  readonly graph: StaticImportGraph
  /** The modules webpack's entries resolve to */
  readonly entries: string[]
  /** The routes atom hoisting counts, `computeFileReach` entries */
  readonly routes: string[]
}

export interface DevupUIWebpackPluginOptions extends Pick<
  DevupUIBasePluginOptions,
  'engine' | 'licenseBanner' | 'stylex'
> {
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
  /**
   * Atom-level route-aware hoisting threshold. Defaults to 2; 0 disables it.
   * Positive thresholds are clamped to >= 2. Canonical buckets reached by at
   * least this many routes use global short counter names in devup-ui.css;
   * other buckets keep per-file counter names in their per-route chunks.
   * Fewer than two routes yields no hoisted plan. Ignored when singleCss is true.
   *
   * Composes with single-importer collapse: files used by exactly one importer
   * still merge into that importer's bucket (deduplicating their identical
   * atoms), and atom hoisting then shares atoms across the remaining buckets.
   */
  atomHoist?: number
  /**
   * Import aliases for redirecting imports from other CSS-in-JS libraries
   * Merged with defaults: @emotion/styled, styled-components, @vanilla-extract/css
   * Set to `false` to disable specific aliases
   */
  importAliases?: ImportAliases
  /** Adapter-owned complete extraction set; undefined keeps generic entry reachability. */
  readonly productionPrewarmFiles?: readonly string[]
}

export class DevupUIWebpackPlugin {
  options: Omit<DevupUIWebpackPluginOptions, 'importAliases'>
  sheetFile: string
  classMapFile: string
  fileMapFile: string
  private importAliases: WasmImportAliases = {}
  private readonly normalizedShorthands: Record<string, readonly string[]>
  private readonly styleResolution: StylexStyleResolution
  private readonly compiler: DevupCompiler
  /** The path options as given, resolved against the project root */
  private readonly paths: Pick<
    Partial<DevupUIWebpackPluginOptions>,
    'cssDir' | 'devupFile' | 'distDir' | 'importAliases'
  > & { readonly productionPrewarmFiles?: readonly string[] }
  /** Webpack's `context` once applied; the working directory before */
  private root = process.cwd()

  constructor({
    package: libPackage = '@devup-ui/react',
    devupFile,
    distDir = 'df',
    cssDir,
    watch = false,
    debug = false,
    include = [],
    singleCss = false,
    engine = 'native',
    prefix,
    licenseBanner = true,
    shorthands,
    stylex,
    atomHoist = 2,
    productionPrewarmFiles,
    importAliases: userImportAliases,
  }: Partial<DevupUIWebpackPluginOptions> = {}) {
    this.normalizedShorthands = normalizeShorthands(shorthands ?? {})
    this.styleResolution = normalizeStylexOptions(stylex)
    this.compiler = loadCompiler(engine)
    this.compiler.registerShorthands(this.normalizedShorthands)
    this.compiler.setStylexStyleResolution(this.styleResolution)
    this.paths = {
      cssDir,
      devupFile,
      distDir,
      importAliases: userImportAliases,
      productionPrewarmFiles,
    }

    this.options = {
      package: libPackage,
      cssDir: '',
      devupFile: '',
      distDir,
      watch,
      debug,
      include,
      singleCss,
      engine,
      prefix,
      licenseBanner,
      shorthands,
      atomHoist,
      productionPrewarmFiles: undefined,
    }
    this.sheetFile = ''
    this.classMapFile = ''
    this.fileMapFile = ''
    this.locate(this.root)
  }

  /**
   * Resolve the path options, the tsconfig and the extraction ids against the
   * project root, so a build started from a parent directory (as monorepo
   * tools do) reads and writes the same files as one started from the root
   */
  private locate(root: string) {
    this.root = root
    const {
      cssDir,
      devupFile,
      distDir = 'df',
      productionPrewarmFiles,
    } = this.paths
    this.options = {
      ...this.options,
      distDir: resolveProjectPath(root, distDir),
      cssDir:
        cssDir === undefined
          ? resolve(root, distDir, 'devup-ui')
          : resolveProjectPath(root, cssDir),
      devupFile: resolveDevupFile(root, devupFile),
      productionPrewarmFiles:
        productionPrewarmFiles === undefined
          ? undefined
          : [
              ...new Set(
                productionPrewarmFiles.map((file) =>
                  relative(
                    root,
                    resolve(root, file.replaceAll('\\', '/')),
                  ).replaceAll('\\', '/'),
                ),
              ),
            ].sort(),
    }
    this.importAliases = mergeImportAliases(
      this.paths.importAliases,
      readJsxImportSource(root),
    )
    this.sheetFile = join(this.options.distDir, 'sheet.json')
    this.classMapFile = join(this.options.distDir, 'classMap.json')
    this.fileMapFile = join(this.options.distDir, 'fileMap.json')
  }

  /** The files the project's Tailwind CSS is read from */
  tailwindFiles: string[] = []

  /**
   * devup.json and every config it `extends`, read or not: editing, adding
   * or deleting one reloads the theme
   */
  configFiles: string[] = []

  /**
   * Why the theme is empty in watch mode: every compilation reports it until
   * devup.json loads again.
   */
  themeError: unknown = undefined

  writeDataFiles() {
    const inputs = new Set<string>()
    try {
      const config = loadDevupConfigSync(this.options.devupFile, inputs)
      const theme = config.theme ?? {}

      this.compiler.registerTheme(withTailwindCss(theme, config))
      this.tailwindFiles = tailwindCssFiles(config)
      // Written even when empty, so tokens removed from the theme leave no
      // stale declarations behind.
      writeFileSync(
        join(this.options.distDir, 'theme.d.ts'),
        this.compiler.getThemeInterface(
          ...createThemeInterfaceArgs(this.options.package),
        ),
        { encoding: 'utf-8' },
      )
      this.themeError = undefined
    } catch (error) {
      // A one-shot build fails on a theme it cannot load or register: an
      // empty theme would ship unstyled output. A watcher keeps running, since
      // devup.json may be mid-edit, and reports the error until it is fixed.
      if (!this.options.watch) throw error
      console.error(error)
      this.compiler.registerTheme({})
      this.themeError = error
    } finally {
      this.configFiles = [
        ...new Set([resolve(this.options.devupFile), ...inputs]),
      ]
    }
    if (!existsSync(this.options.cssDir))
      mkdirSync(this.options.cssDir, { recursive: true })
    if (this.options.watch)
      writeFileSync(
        join(this.options.cssDir, 'devup-ui.css'),
        this.compiler.getCss(null, false),
      )
  }

  /**
   * Merge the files only one module imports into its stylesheet (single-importer
   * collapse, always) and, with `atomHoist`, plan the atoms the routes share,
   * from the graph of the files `discover` finds, which it returns. The
   * webpack loader passes the id relative to the project root as the
   * extraction filename, so the graph maps use root-relative keys (keyBy:
   * 'cwd-relative', with the root as `cwd`). Mirrors next-plugin's pre-pass.
   */
  private installGraph(
    discover: () => DiscoveredGraph,
    atomHoist: number | undefined,
  ): DiscoveredGraph | undefined {
    try {
      const discovered = discover()
      const { graph, routes } = discovered
      const cwd = this.root
      const canonicalMap = buildCanonicalMap({
        srcDir: cwd,
        cwd,
        keyBy: 'cwd-relative',
        graph,
      })
      this.compiler.importCanonicalMap(canonicalMap)
      if (atomHoist === undefined) return discovered
      const plan = planAtomHoist(
        canonicalMap,
        computeFileReach({
          srcDir: cwd,
          cwd,
          keyBy: 'cwd-relative',
          graph,
          entries: routes,
        }),
        atomHoist,
      )
      if (plan) {
        this.compiler.importFileRoutes(plan.reachByBucket)
        this.compiler.setAtomHoist(plan.threshold)
      } else {
        console.info(
          '[devup-ui] fewer than 2 routes were detected; atom hoisting is a no-op (single-entry/SPA).',
        )
      }
      return discovered
    } catch (error) {
      if (
        !this.options.watch &&
        this.options.productionPrewarmFiles !== undefined
      )
        throw error
      // Best-effort; on failure canonical() is the identity (no merge) and atom
      // hoisting stays off.
      console.warn(
        setupWarning(
          'webpack import graph',
          error,
          atomHoist === undefined
            ? 'Classes are not merged across files only one module imports.'
            : 'Classes are not merged across files only one module imports, and atom hoisting is off.',
        ),
      )
    }
  }

  /**
   * Extract the source files the entries reach into
   * the shared compiler sheet, in path order, so that a stylesheet built on
   * its first import holds the styles of every one (all collapsed members of a
   * bucket, and the shared base), not just those of the modules webpack
   * happened to build first. Mirrors the loader's `codeExtract` call (same
   * filename keying + options) so re-extraction during compilation is
   * idempotent. Generic entry discovery remains best-effort; an adapter's
   * complete set is authoritative and its read/extraction failures must fail
   * the build.
   */
  private prewarmExtractor(
    discovered: DiscoveredGraph | undefined,
    { resolve: resolveOptions }: Compiler['options'],
  ) {
    try {
      const cwd = this.root
      // The same resolver as the loader's, so imported constants and
      // stylesheets extract to the classes the loader emits
      this.compiler.setModuleResolver(
        createModuleResolver({
          toId: (path) => relative(cwd, path).replaceAll('\\', '/'),
          conditions: esmImportConditions(resolveOptions),
        }),
      )
      const files =
        this.options.productionPrewarmFiles?.map((file) =>
          resolve(cwd, file),
        ) ?? this.reachableFiles(discovered)
      if (this.options.productionPrewarmFiles !== undefined) {
        seedFileNumbers(this.compiler, [...this.options.productionPrewarmFiles])
      }
      for (const file of files) {
        const relativePath = relative(cwd, file).replaceAll('\\', '/')
        let relCssDir = relative(dirname(file), this.options.cssDir).replaceAll(
          '\\',
          '/',
        )
        if (!relCssDir.startsWith('./')) relCssDir = `./${relCssDir}`
        const output = this.compiler.codeExtract(
          relativePath,
          readFileSync(file, 'utf-8'),
          this.options.package,
          relCssDir,
          this.options.singleCss,
          false,
          true,
          this.importAliases,
        )
        output.free()
      }
    } catch (error) {
      if (this.options.productionPrewarmFiles !== undefined) throw error
      // Best-effort warm-up; on failure the css-loader still serves whatever
      // atoms were extracted, matching pre-fix behavior.
      console.warn(
        setupWarning(
          'webpack stylesheet warm-up',
          error,
          'A stylesheet webpack builds before every file it holds is extracted can miss their styles.',
        ),
      )
    }
  }

  /** The files of the discovered graph webpack's entries reach, in path order */
  private reachableFiles(discovered: DiscoveredGraph | undefined): string[] {
    if (!discovered) return []
    return computeReachableFiles({
      srcDir: this.root,
      entries: discovered.entries,
      graph: discovered.graph,
    })
  }

  /**
   * Forget the source files deleted since the last compilation: they are never
   * loaded again, so their styles would otherwise stay in the sheet. The
   * stylesheet modules are built from the sheet on every compilation.
   */
  removeFiles(files: ReadonlySet<string> | undefined) {
    const cwd = this.root
    let removed = false
    for (const file of files ?? [])
      if (this.compiler.removeFile(relative(cwd, file).replaceAll('\\', '/')))
        removed = true
    if (!removed || !this.options.watch) return
    // The next start restores the sheet without the deleted files
    writeFileSync(this.sheetFile, this.compiler.exportSheet())
    writeFileSync(this.classMapFile, this.compiler.exportClassMap())
    writeFileSync(this.fileMapFile, this.compiler.exportFileMap())
  }

  apply(compiler: Compiler) {
    // A build starts from its own options, not from what an earlier build in
    // this process left in the engine
    const endBuild = beginBuild(this.compiler)
    try {
      compiler.hooks.shutdown?.tap('DevupUIWebpackPlugin', endBuild)
      this.initialize(compiler)
    } catch (error) {
      // Webpack cannot return the compiler when apply throws, so no caller can close it.
      endBuild()
      throw error
    }
  }

  private initialize(compiler: Compiler) {
    this.locate(resolve(compiler.options.context ?? process.cwd()))
    this.compiler.setDebug(this.options.debug)
    this.compiler.setPrefix(this.options.prefix ?? null)
    this.compiler.setLicenseBanner(this.options.licenseBanner !== false)
    this.compiler.registerShorthands(this.normalizedShorthands)
    this.compiler.setStylexStyleResolution(this.styleResolution)
    if (!existsSync(this.options.distDir))
      mkdirSync(this.options.distDir, { recursive: true })
    writeFileSync(join(this.options.distDir, '.gitignore'), '*', 'utf-8')
    writeFileSync(
      join(this.options.distDir, 'compat.d.ts'),
      createCompatTypes(this.importAliases),
      'utf-8',
    )

    if (this.options.watch) {
      try {
        // load sheet
        if (existsSync(this.sheetFile))
          this.compiler.importSheet(
            JSON.parse(readFileSync(this.sheetFile, 'utf-8')),
          )
        if (existsSync(this.classMapFile))
          this.compiler.importClassMap(
            JSON.parse(readFileSync(this.classMapFile, 'utf-8')),
          )
        if (existsSync(this.fileMapFile))
          this.compiler.importFileMap(
            JSON.parse(readFileSync(this.fileMapFile, 'utf-8')),
          )
      } catch (error) {
        console.error(error)
        this.compiler.importSheet({})
        this.compiler.importClassMap({})
        this.compiler.importFileMap({})
      }
    }
    this.writeDataFiles()

    // Per-file atom hoisting defaults to 2; 0 disables it, singleCss ignores it.
    // Configured before loaders run codeExtract, so eligible atoms receive
    // global short counter names. The selected compiler instance is shared
    // in-process with the loaders. Composes with single-importer collapse:
    // both keyed by the canonical bucket.
    const atomHoist = this.options.atomHoist
    const atomMode =
      !this.options.singleCss &&
      atomHoist !== undefined &&
      Number.isFinite(atomHoist) &&
      atomHoist > 0
    // One discovery serves the canonical map, the routes and the warm-up, as
    // in the Vite plugin: the reservation's, over the `src` and `app`
    // directories, the closures of webpack's entries and the included
    // packages. It runs here, with a reservation that reserves nothing:
    // compilers that share the engine in one process (Next's server and
    // client) must all install their plan before any of them extracts.
    const discoverOnly = createWebpackReservation(
      this.options,
      this.importAliases,
      { reserveClassSlots() {}, setModuleResolver() {} },
    )
    const discovered = this.installGraph(
      () => {
        let found: ReservationInventory | undefined
        discoverOnly(compiler.options, (inventory) => {
          found = inventory
        })
        if (!found) throw new Error('no files were discovered')
        // The files the loader transforms: an entry webpack resolves into a
        // package (Next's client runtime) adds no route, so every compiler of
        // a project plans the same
        const excluded = createNodeModulesExcludeRegex(this.options.include)
        const graph = reservationGraph(found, (path) => !excluded.test(path))
        return {
          graph,
          entries: found.resolutions.entries.map((entry) => resolve(entry)),
          routes: reservationRoutes(graph, found),
        }
      },
      atomMode ? atomHoist : undefined,
    )

    // Number every file the build can extract in path order, so class
    // prefixes do not depend on which file a worker reaches first. Numbers
    // already handed out (a restored map in watch mode) stay.
    if (this.options.debug)
      try {
        const cwd = this.root
        seedFileNumbers(
          this.compiler,
          collectNumberedFiles({
            roots: [resolve(cwd, 'src')],
            include: this.options.include,
            cwd,
            needles: extractedNeedles(this.options.package, this.importAliases),
            importAliases: this.importAliases,
            toId: (path) => relative(cwd, path).replaceAll('\\', '/'),
          }),
        )
      } catch (error) {
        if (
          !this.options.watch &&
          this.options.productionPrewarmFiles !== undefined
        )
          throw error
        // Best-effort; numbering falls back to arrival order.
        console.warn(
          setupWarning(
            'webpack file numbering',
            error,
            'Stylesheet file numbers follow the order webpack compiles files in, so they can differ between builds of the same source.',
          ),
        )
      }
    // Pre-warm the extractor so the css-loader serves COMPLETE CSS.
    //
    // Webpack builds a stylesheet module ONCE, at its FIRST import: the shared
    // base (global styles of every file) and, under collapse, a bucket's
    // devup-ui-N.css shared by several source files. Files extracted after that
    // would be missing, so the files the entries reach are extracted up front
    // (single shared compiler instance). Re-extraction by the per-file loader is
    // then idempotent (set-based atom dedup). Watch mode rebuilds stylesheets
    // through the files the loaders write instead.
    const reserve = createWebpackReservation(
      this.options,
      this.importAliases,
      this.compiler,
    )
    const reserveBeforeCompile = () => {
      reserve(compiler.options)
      if (this.options.watch) return
      this.prewarmExtractor(discovered, compiler.options)
    }
    compiler.hooks.run.tap(
      'DevupUIWebpackPluginReservation',
      reserveBeforeCompile,
    )
    if (!this.options.watch)
      compiler.hooks.watchRun.tapPromise(
        'DevupUIWebpackPluginReservation',
        async () => {
          this.removeFiles(compiler.removedFiles)
          reserveBeforeCompile()
        },
      )

    if (this.options.watch) {
      compiler.hooks.thisCompilation.tap(
        'DevupUIWebpackPluginWatchCss',
        (compilation) => {
          if (this.themeError !== undefined)
            compilation.errors.push(
              new compiler.webpack.WebpackError(
                `[devup-ui] ${this.themeError instanceof Error ? this.themeError.message : String(this.themeError)}`,
              ),
            )
          reconcileWatchCss(compilation, this.compiler)
        },
      )
      let lastModifiedTime: string | null = null
      compiler.hooks.watchRun.tapPromise('DevupUIWebpackPlugin', async () => {
        this.removeFiles(compiler.removedFiles)
        const stamps: number[] = []
        for (const file of [...this.configFiles, ...this.tailwindFiles]) {
          stamps.push(
            await stat(file).then(
              (stats) => stats.mtimeMs,
              () => 0,
            ),
          )
        }
        const modifiedTime = stamps.join()
        if (lastModifiedTime && lastModifiedTime !== modifiedTime)
          this.writeDataFiles()
        lastModifiedTime = modifiedTime
        reserve(compiler.options)
      })
    }
    compiler.hooks.afterCompile.tap('DevupUIWebpackPlugin', (compilation) => {
      // A config that does not exist yet is watched for being added
      for (const file of this.configFiles)
        if (existsSync(file)) compilation.fileDependencies.add(file)
        else compilation.missingDependencies.add(file)
      for (const file of this.tailwindFiles)
        compilation.fileDependencies.add(resolve(file))
    })
    compiler.options.plugins.push(
      new compiler.webpack.DefinePlugin({
        'process.env.DEVUP_UI_DEFAULT_THEME': JSON.stringify(
          this.compiler.getDefaultTheme(),
        ),
      }),
    )
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
          const basePath = join(this.options.cssDir, 'devup-ui.css')
          const base = this.compiler.getCss(null, true)
          // A file's stylesheet `@import`s the shared base, which the CSS
          // loaders read from disk without passing through this plugin, so it
          // must hold every style extracted so far
          if (!existsSync(basePath) || readFileSync(basePath, 'utf-8') !== base)
            writeFileSync(basePath, base, 'utf-8')
          let stale: string[] = []
          compilation.hooks.finishModules.tap('DevupUIWebpackPlugin', () => {
            if (compiler.watchMode) return
            const changed = new Set(
              [...servedCss(compilation)]
                .filter(
                  ([path, css]) =>
                    this.compiler.getCss(getFileNumByFilename(path), true) !==
                    css,
                )
                .map(([path]) => path),
            )
            if (this.compiler.getCss(null, true) !== base) changed.add(basePath)
            stale = [...changed]
          })
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
              for (const path of stale)
                writeFileSync(
                  path,
                  this.compiler.getCss(getFileNumByFilename(path), true),
                  'utf-8',
                )
              return true
            },
          )
        },
      )
      compiler.hooks.done.tapPromise('DevupUIWebpackPlugin', async (stats) => {
        if (!stats.hasErrors()) {
          const writes = [
            writeFile(
              join(this.options.cssDir, 'devup-ui.css'),
              this.compiler.getCss(null, false),
              'utf-8',
            ),
          ]
          // Prewarmed calls can omit CSS; completion must persist the final buckets.
          for (const path of servedCss(stats.compilation).keys()) {
            const fileNumber = getFileNumByFilename(path)
            if (fileNumber !== null)
              writes.push(
                writeFile(
                  path,
                  this.compiler.getCss(fileNumber, true),
                  'utf-8',
                ),
              )
          }
          await Promise.all(writes)
        }
      })
    }

    const sourceRule = {
      test: SOURCE_FILE_REGEX,
      exclude: createNodeModulesExcludeRegex(
        this.options.include,
        '.mdx.[tj]sx?$',
      ),
      enforce: 'pre',
      use: [
        {
          loader: createRequire(import.meta.url).resolve(
            '@devup-ui/webpack-plugin/loader',
          ),
          options: {
            package: this.options.package,
            engine: this.options.engine,
            root: this.root,
            cssDir: this.options.cssDir,
            sheetFile: this.sheetFile,
            classMapFile: this.classMapFile,
            fileMapFile: this.fileMapFile,
            watch: this.options.watch,
            singleCss: this.options.singleCss,
            importAliases: this.importAliases,
          },
        },
      ],
    } satisfies RuleSetRule
    compiler.options.module.rules.push(
      sourceRule,
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
              engine: this.options.engine,
            },
          },
        ],
      },
      // MDX compiles to JavaScript in a normal-phase loader such as
      // `@mdx-js/loader`: a `post` loader extracts what it compiled.
      {
        ...sourceRule,
        test: /\.mdx$/i,
        exclude: createNodeModulesExcludeRegex(this.options.include),
        enforce: 'post',
      },
    )
  }
}
