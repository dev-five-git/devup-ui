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
  extractedNeedles,
  getFileNumByFilename,
  type ImportAliases,
  loadDevupConfigSync,
  mergeImportAliases,
  normalizeShorthands,
  planAtomHoist,
  readJsxImportSource,
  seedFileNumbers,
  tailwindCssFiles,
  type WasmImportAliases,
  withTailwindCss,
} from '@devup-ui/plugin-utils'
import {
  codeExtract,
  getCss,
  getDefaultTheme,
  getThemeInterface,
  importCanonicalMap,
  importClassMap,
  importFileMap,
  importFileRoutes,
  importSheet,
  registerShorthands,
  registerTheme,
  resetBuildState,
  seedFileMap,
  setAtomHoist,
  setDebug,
  setModuleResolver,
  setPrefix,
} from '@devup-ui/wasm'
import { type Compiler } from 'webpack'

import { createWebpackReservation } from './reservation'
import { servedCss } from './served-css'
import { reconcileWatchCss } from './watch-css'

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
  private importAliases: WasmImportAliases
  private readonly normalizedShorthands: Record<string, readonly string[]>

  constructor({
    package: libPackage = '@devup-ui/react',
    devupFile = 'devup.json',
    distDir = 'df',
    cssDir = resolve(distDir, 'devup-ui'),
    watch = false,
    debug = false,
    include = [],
    singleCss = false,
    prefix,
    shorthands,
    atomHoist = 2,
    productionPrewarmFiles,
    importAliases: userImportAliases,
  }: Partial<DevupUIWebpackPluginOptions> = {}) {
    this.normalizedShorthands = normalizeShorthands(shorthands ?? {})
    registerShorthands(this.normalizedShorthands)
    this.importAliases = mergeImportAliases(
      userImportAliases,
      readJsxImportSource(),
    )

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
      productionPrewarmFiles:
        productionPrewarmFiles === undefined
          ? undefined
          : [
              ...new Set(
                productionPrewarmFiles.map((file) =>
                  relative(
                    process.cwd(),
                    resolve(process.cwd(), file.replaceAll('\\', '/')),
                  ).replaceAll('\\', '/'),
                ),
              ),
            ].sort(),
    }

    this.sheetFile = join(this.options.distDir, 'sheet.json')
    this.classMapFile = join(this.options.distDir, 'classMap.json')
    this.fileMapFile = join(this.options.distDir, 'fileMap.json')
  }

  /** The files the project's Tailwind CSS is read from */
  tailwindFiles: string[] = []

  writeDataFiles() {
    try {
      const config = loadDevupConfigSync(this.options.devupFile)
      const theme = config.theme ?? {}

      registerTheme(withTailwindCss(theme, config))
      this.tailwindFiles = tailwindCssFiles(config)
      const interfaceCode = getThemeInterface(
        ...createThemeInterfaceArgs(this.options.package),
      )

      if (interfaceCode) {
        writeFileSync(join(this.options.distDir, 'theme.d.ts'), interfaceCode, {
          encoding: 'utf-8',
        })
      }
    } catch (error) {
      console.error(error)
      registerTheme({})
    }
    if (!existsSync(this.options.cssDir))
      mkdirSync(this.options.cssDir, { recursive: true })
    if (this.options.watch)
      writeFileSync(
        join(this.options.cssDir, 'devup-ui.css'),
        getCss(null, false),
      )
  }

  /**
   * Extract the source files under `src` that `entries` reach into the shared
   * WASM sheet, in path order, so that a stylesheet built on its first import
   * holds the styles of every one (all collapsed members of a bucket, and the
   * shared base), not just those of the modules webpack happened to build
   * first. Mirrors the loader's `codeExtract` call (same filename keying +
   * options) so re-extraction during compilation is idempotent. Generic entry
   * discovery remains best-effort; an adapter's complete set is authoritative
   * and its read/extraction failures must fail the build.
   */
  private prewarmExtractor(entries: string[]) {
    try {
      const cwd = process.cwd()
      // The same resolver as the loader's, so imported constants and
      // stylesheets extract to the classes the loader emits
      setModuleResolver(
        createModuleResolver({
          toId: (path) => relative(cwd, path).replaceAll('\\', '/'),
        }),
      )
      const files =
        this.options.productionPrewarmFiles?.map((file) =>
          resolve(cwd, file),
        ) ??
        computeReachableFiles({
          srcDir: resolve(cwd, 'src'),
          tsconfigPath: resolve(cwd, 'tsconfig.json'),
          entries,
        })
      if (this.options.productionPrewarmFiles !== undefined) {
        seedFileNumbers({ seedFileMap }, [
          ...this.options.productionPrewarmFiles,
        ])
      }
      for (const file of files) {
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
      }
    } catch (error) {
      if (this.options.productionPrewarmFiles !== undefined) throw error
      // Best-effort warm-up; on failure the css-loader still serves whatever
      // atoms were extracted, matching pre-fix behavior.
    }
  }

  apply(compiler: Compiler) {
    // A build starts from its own options, not from what an earlier build in
    // this process left in the engine
    const endBuild = beginBuild({ resetBuildState })
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
    setDebug(this.options.debug)
    setPrefix(this.options.prefix ?? null)
    registerShorthands(this.normalizedShorthands)
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

    if (this.options.watch) {
      try {
        // load sheet
        if (existsSync(this.sheetFile))
          importSheet(JSON.parse(readFileSync(this.sheetFile, 'utf-8')))
        if (existsSync(this.classMapFile))
          importClassMap(JSON.parse(readFileSync(this.classMapFile, 'utf-8')))
        if (existsSync(this.fileMapFile))
          importFileMap(JSON.parse(readFileSync(this.fileMapFile, 'utf-8')))
      } catch (error) {
        console.error(error)
        importSheet({})
        importClassMap({})
        importFileMap({})
      }
    }
    this.writeDataFiles()

    // Per-file atom hoisting defaults to 2; 0 disables it, singleCss ignores it.
    // Configured before loaders run codeExtract (apply() is synchronous), so
    // eligible atoms receive global short counter names. The WASM
    // instance is shared in-process with the loaders. Composes with
    // single-importer collapse: both keyed by the canonical bucket. The webpack
    // loader passes relative(process.cwd(), id) as the extraction filename, so
    // the graph maps use cwd-relative keys (keyBy: 'cwd-relative').
    const atomHoist = this.options.atomHoist
    const atomMode =
      !this.options.singleCss &&
      atomHoist !== undefined &&
      Number.isFinite(atomHoist) &&
      atomHoist > 0
    // Single-importer collapse ALWAYS runs: files used by exactly one importer
    // merge into that importer's bucket, deduplicating their identical atoms.
    // The canonical map is built + imported unconditionally; only atom HOISTING
    // composes on top in effective atom mode. Mirrors next-plugin's pre-pass.
    try {
      const srcDir = resolve(process.cwd(), 'src')
      const tsconfigPath = resolve(process.cwd(), 'tsconfig.json')
      const cwd = process.cwd()
      const canonicalMap = buildCanonicalMap({
        srcDir,
        tsconfigPath,
        cwd,
        keyBy: 'cwd-relative',
      })
      importCanonicalMap(canonicalMap)

      if (atomMode) {
        const fileReach = computeFileReach({
          srcDir,
          tsconfigPath,
          cwd,
          keyBy: 'cwd-relative',
        })
        const plan = planAtomHoist(canonicalMap, fileReach, atomHoist)
        if (plan) {
          importFileRoutes(plan.reachByBucket)
          setAtomHoist(plan.threshold)
        } else {
          console.info(
            '[devup-ui] fewer than 2 routes were detected; atom hoisting is a no-op (single-entry/SPA).',
          )
        }
      }
    } catch (error) {
      if (
        !this.options.watch &&
        this.options.productionPrewarmFiles !== undefined
      )
        throw error
      // Best-effort; on failure canonical() is the identity (no merge) and atom
      // hoisting stays off.
    }

    // Number every file the build can extract in path order, so class
    // prefixes do not depend on which file a worker reaches first. Numbers
    // already handed out (a restored map in watch mode) stay.
    if (this.options.debug)
      try {
        const cwd = process.cwd()
        seedFileNumbers(
          { seedFileMap },
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
    const reserve = createWebpackReservation(this.options, this.importAliases)
    const reserveBeforeCompile = () => {
      reserve(compiler.options)
      if (this.options.watch) return
      const { entry, context = process.cwd() } = compiler.options
      this.prewarmExtractor(
        typeof entry === 'function'
          ? []
          : Object.values(entry ?? {}).flatMap(({ import: requests = [] }) =>
              requests.map((request) => resolve(context, request)),
            ),
      )
    }
    compiler.hooks.run.tap(
      'DevupUIWebpackPluginReservation',
      reserveBeforeCompile,
    )
    if (!this.options.watch)
      compiler.hooks.watchRun.tapPromise(
        'DevupUIWebpackPluginReservation',
        async () => {
          reserveBeforeCompile()
        },
      )

    if (this.options.watch) {
      compiler.hooks.thisCompilation.tap(
        'DevupUIWebpackPluginWatchCss',
        reconcileWatchCss,
      )
      let lastModifiedTime: string | null = null
      compiler.hooks.watchRun.tapPromise('DevupUIWebpackPlugin', async () => {
        const stamps: number[] = []
        if (existsDevup)
          stamps.push((await stat(this.options.devupFile)).mtimeMs)
        for (const file of this.tailwindFiles) {
          stamps.push(
            await stat(file).then(
              (stats) => stats.mtimeMs,
              () => 0,
            ),
          )
        }
        if (stamps.length > 0) {
          const modifiedTime = stamps.join()
          if (lastModifiedTime && lastModifiedTime !== modifiedTime)
            this.writeDataFiles()

          lastModifiedTime = modifiedTime
        }
        reserve(compiler.options)
      })
    }
    if (existsDevup || this.tailwindFiles.length > 0)
      compiler.hooks.afterCompile.tap('DevupUIWebpackPlugin', (compilation) => {
        if (existsDevup)
          compilation.fileDependencies.add(resolve(this.options.devupFile))
        for (const file of this.tailwindFiles)
          compilation.fileDependencies.add(resolve(file))
      })
    compiler.options.plugins.push(
      new compiler.webpack.DefinePlugin({
        'process.env.DEVUP_UI_DEFAULT_THEME': JSON.stringify(getDefaultTheme()),
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
          const base = getCss(null, true)
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
                    getCss(getFileNumByFilename(path), true) !== css,
                )
                .map(([path]) => path),
            )
            if (getCss(null, true) !== base) changed.add(basePath)
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
                  getCss(getFileNumByFilename(path), true),
                  'utf-8',
                )
              return true
            },
          )
        },
      )
      compiler.hooks.done.tapPromise('DevupUIWebpackPlugin', async (stats) => {
        if (!stats.hasErrors()) {
          // write css file
          await writeFile(
            join(this.options.cssDir, 'devup-ui.css'),
            getCss(null, false),
            'utf-8',
          )
        }
      })
    }

    compiler.options.module.rules.push(
      {
        test: /\.(tsx|ts|js|mjs|jsx)$/,
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
  }
}
