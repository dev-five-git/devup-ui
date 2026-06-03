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
import type { RsbuildPlugin } from '@rsbuild/core'

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
  /**
   * Atom-level route-aware hoisting threshold (min routes sharing an atom for it
   * to hoist into the shared devup-ui.css; clamped to >= 2; omit to disable).
   * Opt-in: when set, single-importer collapse + atom hoisting are enabled and
   * per-route CSS is served via getCss(fileNum). "Routes" are inferred from the
   * import graph (entry points and dynamic-import targets). For a single-entry
   * SPA (routeCount < 2) it is a no-op.
   *
   * KNOWN LIMITATION (rsbuild MPA): hoisted atoms get a single class name
   * (correct), but rspack's default chunk strategy INLINES the shared base CSS
   * into each entry bundle, so the base is duplicated across entries rather than
   * emitted as one shared chunk. Rendering is correct; the cross-entry dedup
   * benefit is not yet realized. Deduping requires an rspack `splitChunks`
   * cacheGroup (type `css/mini-extract`, `minChunks >= 2`) — tracked as a
   * follow-up. See the cssDir transform TODO below.
   */
  atomHoist?: number
  /**
   * Import aliases for redirecting imports from other CSS-in-JS libraries
   * Merged with defaults: @emotion/styled, styled-components, @vanilla-extract/css
   * Set to `false` to disable specific aliases
   */
  importAliases?: ImportAliases
}

let globalCss = ''

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
  atomHoist,
  importAliases: userImportAliases,
}: Partial<DevupUIRsbuildPluginOptions> = {}): RsbuildPlugin => {
  const importAliases = mergeImportAliases(userImportAliases)

  return {
    name: 'devup-ui-rsbuild-plugin',
    async setup(api) {
      setDebug(debug)
      if (prefix) {
        setPrefix(prefix)
      }

      if (!existsSync(distDir)) await mkdir(distDir, { recursive: true })
      await writeFile(join(distDir, '.gitignore'), '*', 'utf-8')

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
          const reachByBucket: Record<string, number[]> = {}
          for (const [file, ids] of Object.entries(fileReach)) {
            const bucket = canonicalMap[file] ?? file
            if (bucket === '@global') continue
            const set = (reachByBucket[bucket] ??= [])
            for (const id of ids) if (!set.includes(id)) set.push(id)
          }
          const routeCount = new Set(Object.values(fileReach).flat()).size
          if (routeCount >= 2) {
            importFileRoutes(reachByBucket)
            setAtomHoist(Math.max(2, atomHoist))
          }
        } catch {
          // Best-effort; on failure atom hoisting stays off (identity).
        }
      }

      api.transform(
        {
          test: cssDir,
        },
        ({ resourcePath }) => {
          // Non-atom: keep the existing single-string behavior (no regression).
          if (!atomMode) return globalCss
          // Atom mode: serve the route-specific chunk and have it @import the
          // shared base (devup-ui.css) so hoisted atoms load ONCE and are not
          // inlined per chunk. The base file itself imports nothing.
          // Route chunk and base are SEPARATE modules (the transformed entry
          // code imports both via import_main_css).
          // TODO(atom-B follow-up): rspack's default chunk strategy inlines the
          // shared base CSS into each entry bundle (duplicated across MPA
          // entries). Add an rspack `splitChunks` cacheGroup (type
          // 'css/mini-extract', minChunks >= 2) to emit the base as one shared
          // chunk and realize the cross-entry dedup benefit.
          return getCss(getFileNumByFilename(basename(resourcePath)), false)
        },
      )

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
        return config
      })

      api.transform(
        {
          test: /\.(tsx|ts|js|mjs|jsx)$/,
        },
        async ({ code, resourcePath }) => {
          if (createNodeModulesExcludeRegex(include).test(resourcePath))
            return code
          // Atom mode mirrors vite: the entry CODE imports the shared base
          // (import_main_css_in_code=true) so rspack emits devup-ui.css once and
          // links it from every entry (hoisted atoms shared, not inlined). A
          // relative cssDir is required for that code import to resolve, and the
          // extraction filename is POSIX-normalized to match the absolute-keyed
          // canonical map / FILE_ROUTES. Non-atom keeps the prior behavior.
          let relCssDir = relative(dirname(resourcePath), cssDir).replaceAll(
            '\\',
            '/',
          )
          if (!relCssDir.startsWith('./')) relCssDir = `./${relCssDir}`
          const {
            code: retCode,
            css = '',
            map,
            cssFile,
            updatedBaseStyle,
          } = codeExtract(
            atomMode ? resourcePath.replaceAll('\\', '/') : resourcePath,
            code,
            libPackage,
            atomMode ? relCssDir : cssDir,
            singleCss,
            atomMode,
            !atomMode,
            importAliases,
          )
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
            if (globalCss.length < css.length) globalCss = css
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
