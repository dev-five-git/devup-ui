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
  planAtomHoist,
  seedFileNumbers,
  type WasmImportAliases,
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
  setNamingRoot,
  setPrefix,
} from '@devup-ui/wasm'
import { type Compiler } from 'webpack'

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
    atomHoist,
    importAliases: userImportAliases,
  }: Partial<DevupUIWebpackPluginOptions> = {}) {
    registerShorthands(shorthands ?? {})
    this.importAliases = mergeImportAliases(userImportAliases)

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
      atomHoist,
    }

    this.sheetFile = join(this.options.distDir, 'sheet.json')
    this.classMapFile = join(this.options.distDir, 'classMap.json')
    this.fileMapFile = join(this.options.distDir, 'fileMap.json')
  }

  writeDataFiles() {
    try {
      const config = loadDevupConfigSync(this.options.devupFile)
      const theme = config.theme ?? {}

      registerTheme(theme)
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
   * options) so re-extraction during compilation is idempotent. Best-effort:
   * extraction errors are swallowed so a single bad file never breaks the
   * build, and a stylesheet still missing styles is rebuilt by another pass.
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
      for (const file of computeReachableFiles({
        srcDir: resolve(cwd, 'src'),
        tsconfigPath: resolve(cwd, 'tsconfig.json'),
        entries,
      })) {
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
    } catch {
      // Best-effort warm-up; on failure the css-loader still serves whatever
      // atoms were extracted, matching pre-fix behavior.
    }
  }

  apply(compiler: Compiler) {
    // A build starts from its own options, not from what an earlier build in
    // this process left in the engine
    const endBuild = beginBuild({ resetBuildState })
    compiler.hooks.shutdown?.tap('DevupUIWebpackPlugin', endBuild)
    setDebug(this.options.debug)
    setPrefix(this.options.prefix ?? null)
    setNamingRoot(compiler.options.context ?? process.cwd(), process.cwd())
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
            '[devup-ui] atomHoist is set but fewer than 2 routes were detected; atom hoisting is a no-op (single-entry/SPA).',
          )
        }
      }
    } catch {
      // Best-effort; on failure canonical() is the identity (no merge) and atom
      // hoisting stays off.
    }

    // Number every file the build can extract in path order, so class
    // prefixes do not depend on which file a worker reaches first. Numbers
    // already handed out (a restored map in watch mode) stay.
    try {
      const cwd = process.cwd()
      seedFileNumbers(
        { seedFileMap },
        collectNumberedFiles({
          roots: [resolve(cwd, 'src')],
          include: this.options.include,
          cwd,
          needles: extractedNeedles(this.options.package, this.importAliases),
          toId: (path) => relative(cwd, path).replaceAll('\\', '/'),
        }),
      )
    } catch {
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
    if (!this.options.watch) {
      const { entry, context = process.cwd() } = compiler.options
      this.prewarmExtractor(
        typeof entry === 'function'
          ? []
          : Object.values(entry ?? {}).flatMap(({ import: requests = [] }) =>
              requests.map((request) => resolve(context, request)),
            ),
      )
    }

    if (this.options.watch) {
      let lastModifiedTime: number | null = null
      compiler.hooks.watchRun.tapPromise('DevupUIWebpackPlugin', async () => {
        if (existsDevup) {
          const stats = await stat(this.options.devupFile)

          const modifiedTime = stats.mtimeMs
          if (lastModifiedTime && lastModifiedTime !== modifiedTime)
            this.writeDataFiles()

          lastModifiedTime = modifiedTime
        }
      })
    }
    if (existsDevup)
      compiler.hooks.afterCompile.tap('DevupUIWebpackPlugin', (compilation) => {
        compilation.fileDependencies.add(resolve(this.options.devupFile))
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
