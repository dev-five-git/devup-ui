import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { stat, writeFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { join, resolve } from 'node:path'

import {
  buildCanonicalMap,
  computeFileReach,
  createNodeModulesExcludeRegex,
  createThemeInterfaceArgs,
  type ImportAliases,
  loadDevupConfigSync,
  mergeImportAliases,
  planAtomHoist,
  type WasmImportAliases,
} from '@devup-ui/plugin-utils'
import {
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
  setDebug,
  setPrefix,
} from '@devup-ui/wasm'
import { type Compiler } from 'webpack'

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
    atomHoist,
    importAliases: userImportAliases,
  }: Partial<DevupUIWebpackPluginOptions> = {}) {
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

  apply(compiler: Compiler) {
    setDebug(this.options.debug)
    if (this.options.prefix) {
      setPrefix(this.options.prefix)
    }
    const existsDevup = existsSync(this.options.devupFile)
    // read devup.json
    if (!existsSync(this.options.distDir))
      mkdirSync(this.options.distDir, { recursive: true })
    writeFileSync(join(this.options.distDir, '.gitignore'), '*', 'utf-8')

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
    if (atomMode) {
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
      } catch {
        // Best-effort; on failure atom hoisting stays off (identity).
      }
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
