import { existsSync } from 'node:fs'
import { mkdir, writeFile } from 'node:fs/promises'
import { basename, dirname, join, relative, resolve } from 'node:path'

import {
  buildCanonicalMap,
  computeFileReach,
  createNodeModulesExcludeRegex,
  createThemeInterfaceArgs,
  getFileNumByFilename,
  type ImportAliases,
  loadDevupConfig,
  mergeImportAliases,
  planAtomHoist,
} from '@devup-ui/plugin-utils'
import {
  codeExtract,
  getCss,
  getDefaultTheme,
  getThemeInterface,
  importCanonicalMap,
  importFileRoutes,
  registerTheme,
  setAtomHoist,
  setDebug,
  setPrefix,
} from '@devup-ui/wasm'
import type { ModuleNode, PluginOption, UserConfig } from 'vite'

export interface DevupUIPluginOptions {
  package: string
  cssDir: string
  devupFile: string
  distDir: string
  extractCss: boolean
  debug: boolean
  include: string[]
  singleCss: boolean
  prefix?: string
  /**
   * Atom-level route-aware hoisting threshold (min routes sharing an atom for
   * it to hoist into the shared devup-ui.css; clamped to >= 2; omit to disable).
   * Opt-in: when set, single-importer collapse + atom hoisting are enabled for
   * this build. "Routes" are inferred from the import graph (entry points and
   * dynamic-import targets).
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
  options: Omit<DevupUIPluginOptions, 'extractCss' | 'debug' | 'include'>,
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

export function DevupUI({
  package: libPackage = '@devup-ui/react',
  devupFile = 'devup.json',
  distDir = 'df',
  cssDir = resolve(distDir, 'devup-ui'),
  extractCss = true,
  debug = false,
  include = [],
  singleCss = false,
  prefix,
  atomHoist,
  importAliases: userImportAliases,
}: Partial<DevupUIPluginOptions> = {}): PluginOption {
  setDebug(debug)
  if (prefix) {
    setPrefix(prefix)
  }
  const importAliases = mergeImportAliases(userImportAliases)
  const cssMap = new Map()
  return {
    name: 'devup-ui',
    async configResolved(config) {
      if (!existsSync(distDir)) await mkdir(distDir, { recursive: true })
      await writeFile(join(distDir, '.gitignore'), '*', 'utf-8')
      await writeDataFiles({
        package: libPackage,
        cssDir,
        devupFile,
        distDir,
        singleCss,
      })

      // Atom-level hoisting (opt-in via `atomHoist`). Configured BEFORE any
      // transform so atoms receive global (shared) class names. Composes with
      // single-importer collapse: both are keyed by the canonical bucket. Vite
      // passes the ABSOLUTE module id to codeExtract, so the graph maps use
      // absolute keys (keyBy: 'absolute') to match the engine's bucket keys.
      const atomMode =
        atomHoist !== undefined && Number.isFinite(atomHoist) && atomHoist > 0
      if (atomMode) {
        try {
          const root = config.root ?? process.cwd()
          const srcDir = resolve(root, 'src')
          const tsconfigPath = resolve(root, 'tsconfig.json')
          // C: prefer the bundler's real JS entries; fall back to the heuristic
          // (files with no importer) when input is html-only / unavailable.
          const input = config.build?.rollupOptions?.input
          const rawEntries =
            typeof input === 'string'
              ? [input]
              : Array.isArray(input)
                ? input
                : input && typeof input === 'object'
                  ? Object.values(input)
                  : []
          const entries = rawEntries
            .filter((e): e is string => typeof e === 'string')
            .filter((e) => /\.(tsx|ts|jsx|js|mjs)$/i.test(e))
            .map((e) => resolve(root, e))

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
            entries: entries.length > 0 ? entries : undefined,
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
    },
    config() {
      const theme = getDefaultTheme()
      const define: Record<string, string> = {}
      if (theme) {
        define['process.env.DEVUP_UI_DEFAULT_THEME'] = JSON.stringify(theme)
      }
      const ret: Omit<UserConfig, 'plugins'> = {
        server: {
          watch: {
            ignored: [`!${devupFile}`],
          },
        },
        define,
        optimizeDeps: {
          exclude: [...include, '@devup-ui/components', '@devup-editor/react'],
        },
        ssr: {
          noExternal: [...include, /@devup-ui/, /@devup-editor/],
        },
      }
      if (extractCss) {
        ret.build = {
          rollupOptions: {
            output: {
              manualChunks(id) {
                // merge devup css files
                const fileName = basename(id).split('?')[0]
                if (/devup-ui(-\d+)?\.css$/.test(fileName)) {
                  return fileName
                }
              },
            },
          },
        }
      }
      return ret
    },
    apply() {
      return true
    },
    async watchChange(id) {
      if (resolve(id) === resolve(devupFile) && existsSync(devupFile)) {
        try {
          await writeDataFiles({
            package: libPackage,
            cssDir,
            devupFile,
            distDir,
            singleCss,
          })
        } catch (error) {
          console.error(error)
        }
      }
    },
    async handleHotUpdate({ file, server, modules, timestamp }) {
      if (resolve(file) !== resolve(devupFile) || !existsSync(devupFile)) {
        return
      }

      await writeDataFiles({
        package: libPackage,
        cssDir,
        devupFile,
        distDir,
        singleCss,
      })

      const invalidatedModules = new Set<ModuleNode>()
      for (const mod of modules) {
        server.moduleGraph.invalidateModule(
          mod,
          invalidatedModules,
          timestamp,
          true,
        )
      }
      server.ws.send({ type: 'full-reload' })
      return []
    },
    resolveId(id, importer) {
      const fileName = basename(id).split('?')[0]
      if (
        /devup-ui(-\d+)?\.css$/.test(fileName) &&
        resolve(importer ? join(dirname(importer), id) : id) ===
          resolve(join(cssDir, fileName))
      ) {
        return join(
          cssDir,
          `${fileName}?t=${
            Date.now().toString() +
            (cssMap.get(getFileNumByFilename(fileName))?.length ?? 0)
          }`,
        )
      }
    },
    load(id) {
      const fileName = basename(id).split('?')[0]
      if (/devup-ui(-\d+)?\.css$/.test(fileName)) {
        const fileNum = getFileNumByFilename(fileName)
        const css = getCss(fileNum, false)
        cssMap.set(fileNum, css)
        return css
      }
    },
    enforce: 'pre',
    async transform(code, id) {
      if (!extractCss) return

      const fileName = id.split('?')[0]
      if (!/\.(tsx|ts|js|mjs|jsx)$/i.test(fileName)) return
      if (createNodeModulesExcludeRegex(include).test(fileName)) {
        return
      }

      let rel = relative(dirname(id), cssDir).replaceAll('\\', '/')
      if (!rel.startsWith('./')) rel = `./${rel}`

      const {
        code: retCode,
        css = '',
        map,
        cssFile,
        updatedBaseStyle,
        // import main css in code
      } = codeExtract(
        fileName,
        code,
        libPackage,
        rel,
        singleCss,
        true,
        false,
        importAliases,
      )
      const promises: Promise<void>[] = []

      if (updatedBaseStyle) {
        // update base style
        promises.push(
          writeFile(join(cssDir, 'devup-ui.css'), getCss(null, false), 'utf-8'),
        )
      }

      if (cssFile) {
        const fileNum = getFileNumByFilename(cssFile)
        const prevCss = cssMap.get(fileNum)
        if (prevCss && prevCss.length < css.length) cssMap.set(fileNum, css)
        promises.push(
          writeFile(
            join(cssDir, basename(cssFile)),
            `/* ${id} ${Date.now()} */`,
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
    async generateBundle(_options, bundle) {
      if (!extractCss) return

      const cssFile = Object.keys(bundle).find(
        (file) => bundle[file].name === 'devup-ui.css',
      )
      if (cssFile && 'source' in bundle[cssFile]) {
        bundle[cssFile].source = cssMap.get(null) ?? ''
      }
    },
  }
}
