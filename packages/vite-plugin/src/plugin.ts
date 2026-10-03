import { existsSync } from 'node:fs'
import { mkdir, writeFile } from 'node:fs/promises'
import { basename, dirname, join, relative, resolve } from 'node:path'

import {
  beginBuild,
  buildCanonicalMap,
  collectNumberedFiles,
  computeFileReach,
  createCompatTypes,
  createModuleResolver,
  createNodeModulesExcludeRegex,
  createStateWriter,
  createThemeInterfaceArgs,
  type CustomShorthands,
  extractedNeedles,
  getFileNumByFilename,
  type ImportAliases,
  loadDevupConfig,
  mergeImportAliases,
  planAtomHoist,
  seedFileNumbers,
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
import type {
  EnvironmentModuleNode,
  ModuleNode,
  PluginOption,
  UserConfig,
} from 'vite'

/**
 * CSS entry files emitted by devup-ui: `devup-ui.css`, `devup-ui-3.css`, ...
 *
 * Anchored at BOTH ends and matched against a bare file name. Without `^` it
 * also accepts an app's own `vendor-devup-ui.css`, which `generateBundle` would
 * then overwrite with the devup sheet.
 */
const DEVUP_CSS_FILE_RE = /^devup-ui(-\d+)?\.css$/

const SOURCE_DIR_CANDIDATES = ['src', 'app']

/**
 * Source roots to scan, or an empty list when none of the conventional layouts
 * are present. Scanning the project root instead would sweep in config files
 * and other never-transformed modules, so callers skip the scan entirely rather
 * than guess.
 */
function resolveSourceDirs(root: string): string[] {
  return SOURCE_DIR_CANDIDATES.map((dir) => resolve(root, dir)).filter((dir) =>
    existsSync(dir),
  )
}

/**
 * Names each devup CSS module after its own file, so every module is emitted
 * once and shared by all of its importers.
 */
function getDevupCssChunkName(id: string): string | undefined {
  const fileName = basename(id).split('?')[0]
  return DEVUP_CSS_FILE_RE.test(fileName) ? fileName : undefined
}

/**
 * Subset of the plugin context Vite binds to the `config` hook. Vite >= 6.1
 * exposes `meta.viteVersion`; a Rolldown-powered Vite also exposes
 * `meta.rolldownVersion`.
 */
interface ConfigHookMeta {
  viteVersion?: string
  rolldownVersion?: string
}

interface ViteOutputWithMetadata {
  viteMetadata?: {
    importedCss?: Set<string>
  }
}

/**
 * Vite merges a plugin's `config()` result over the user's, replacing function
 * values outright, so returning a bare `manualChunks` silently drops one the
 * app already had. Rolldown's `codeSplitting.groups` needs no equivalent —
 * arrays are concatenated, not replaced.
 */
function getUserManualChunks(userConfig: UserConfig | undefined) {
  const output = userConfig?.build?.rollupOptions?.output
  const first = Array.isArray(output) ? output[0] : output
  return typeof first?.manualChunks === 'function'
    ? first.manualChunks
    : undefined
}

/**
 * Build options that merge devup-ui CSS modules into shared chunks.
 *
 * Rollup only understands `output.manualChunks`. Rolldown (Vite 8) removed the
 * object form and *silently ignores* the function form as soon as anything sets
 * `output.codeSplitting` — which framework plugins do per environment (vinext
 * sets it for its client and rsc environments, so only its ssr environment
 * still honors `manualChunks`). The shared CSS then gets copied into every
 * route chunk instead of being emitted once.
 *
 * `output.codeSplitting.groups` is Rolldown's replacement, and merges with the
 * groups a framework plugin already registered.
 *
 * @see https://vite.dev/guide/migration.html#removed-object-form-build-rollupoptions-output-manualchunks-and-deprecate-function-form-one
 * @see https://rolldown.rs/in-depth/manual-code-splitting
 */
function createCssChunkBuildOptions(
  meta: ConfigHookMeta | undefined,
  userConfig?: UserConfig,
): UserConfig['build'] {
  if (!meta?.rolldownVersion) {
    const userManualChunks = getUserManualChunks(userConfig)
    return {
      rollupOptions: {
        output: {
          manualChunks(id, ...rest) {
            return getDevupCssChunkName(id) ?? userManualChunks?.(id, ...rest)
          },
        },
      },
    }
  }
  const output = {
    codeSplitting: {
      groups: [
        {
          name: (id: string) => getDevupCssChunkName(id) ?? null,
          // Opt out of any framework-level `minSize` / `minShareCount`
          // fallback, which would otherwise fold these small CSS chunks back
          // into every route chunk that imports them.
          minSize: 0,
          minShareCount: 1,
        },
      ],
    },
  }
  // Vite 8 renamed `build.rollupOptions` to `build.rolldownOptions`. Older
  // Rolldown-powered builds (rolldown-vite on Vite 7) keep the old name, and
  // supplying both would make Vite drop one of them.
  return Number.parseInt(meta.viteVersion ?? '', 10) >= 8
    ? { rolldownOptions: { output } }
    : { rollupOptions: { output } }
}

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
  shorthands?: CustomShorthands
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
  // Sequential: writing into cssDir concurrently with its own mkdir loses the
  // race on a cold start (no `df/`) and fails the build with ENOENT.
  if (!existsSync(options.cssDir)) {
    await mkdir(options.cssDir, { recursive: true })
  }
  if (!options.singleCss) {
    await writeFile(join(options.cssDir, 'devup-ui.css'), getCss(null, false))
  }
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
  shorthands,
  atomHoist,
  importAliases: userImportAliases,
}: Partial<DevupUIPluginOptions> = {}): PluginOption {
  // A build starts from its own options: whatever an earlier build in this
  // process left in the engine (prefix, hoisting, routes, buckets, numbers,
  // styles) is gone unless another build is still running.
  const endBuild = beginBuild({ resetBuildState })
  registerShorthands(shorthands ?? {})
  setDebug(debug)
  setPrefix(prefix ?? null)
  const importAliases = mergeImportAliases(userImportAliases)
  const cssMap = new Map()
  let serverBundleToForward: Record<string, ViteOutputWithMetadata> | undefined
  let isServe = false
  // The dev server watches cssDir, so every write is an update signal. A
  // module transformed again writes its sheet again, and the reload that
  // signal causes transforms it once more: signal only a changed sheet.
  const writtenCss = new Map<string, string>()
  const stateWriter = createStateWriter((path, content, encoding) =>
    encoding ? writeFile(path, content, encoding) : writeFile(path, content),
  )
  function writeCssFile(fileName: string, css: string): Promise<void> {
    if (writtenCss.get(fileName) === css) return Promise.resolve()
    writtenCss.set(fileName, css)
    return stateWriter.write(join(cssDir, fileName), css, 'utf-8')
  }
  return {
    name: 'devup-ui',
    // The WASM sheet and transform state are intentionally shared. Vite
    // otherwise recreates this plugin for every environment build, which makes
    // each environment independently emit the same CSS asset.
    sharedDuringBuild: true,
    async configResolved(config) {
      isServe = config?.command === 'serve'
      const projectRoot = config?.root ?? process.cwd()
      // Vite ids are POSIX absolute paths
      setModuleResolver(
        createModuleResolver({
          cwd: projectRoot,
          toId: (path) => path.replaceAll('\\', '/'),
        }),
      )
      const sourceDirs = resolveSourceDirs(projectRoot)
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

      // Atom-level hoisting (opt-in via `atomHoist`). Configured BEFORE any
      // transform so atoms receive global (shared) class names. Composes with
      // single-importer collapse: both are keyed by the canonical bucket. Vite
      // passes the ABSOLUTE module id to codeExtract, so the graph maps use
      // absolute keys (keyBy: 'absolute') to match the engine's bucket keys.
      const atomMode =
        atomHoist !== undefined && Number.isFinite(atomHoist) && atomHoist > 0
      if (atomMode) {
        try {
          const root = projectRoot
          // App Router projects keep their sources in `app/`, so a hardcoded
          // `src/` made the whole pre-pass a silent no-op for them.
          const srcDir = sourceDirs[0] ?? resolve(root, 'src')
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
      try {
        // Numbers come from the sorted paths of every file the build can
        // extract (source and included packages), not from arrival order.
        // Files numbered before keep their numbers, so a later pass in the
        // dev server only numbers new files after the existing ones.
        seedFileNumbers(
          { seedFileMap },
          collectNumberedFiles({
            roots: sourceDirs,
            include,
            cwd: projectRoot,
            needles: extractedNeedles(libPackage, importAliases),
          }),
        )
      } catch {
        // Best-effort; on failure numbering falls back to arrival order.
      }
    },
    config(this: { meta?: ConfigHookMeta } | void, userConfig: UserConfig) {
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
        ret.build = createCssChunkBuildOptions(this?.meta, userConfig)
      }
      return ret
    },
    apply() {
      return true
    },
    closeBundle() {
      endBuild()
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
    // Runs once per environment. Vite 6+ ignores `handleHotUpdate` on a plugin
    // that defines this hook, so the devup.json reload lives here as well.
    async hotUpdate({ file, modules, timestamp }) {
      const { environment } = this
      if (environment.config.consumer === 'server') {
        // A module runner cannot apply CSS, so Vite answers a sheet change
        // with a full reload: the render restarts mid-request, and the modules
        // it transforms again write their sheets again. Server environments
        // only reference sheets by URL; the client refreshes their contents.
        const fileName = basename(file)
        return DEVUP_CSS_FILE_RE.test(fileName) &&
          resolve(file) === resolve(cssDir, fileName)
          ? []
          : undefined
      }
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

      const invalidatedModules = new Set<EnvironmentModuleNode>()
      for (const mod of modules) {
        environment.moduleGraph.invalidateModule(
          mod,
          invalidatedModules,
          timestamp,
          true,
        )
      }
      environment.hot.send({ type: 'full-reload' })
      return []
    },
    // Vite 5 fallback: Vite 6+ does not call this hook when `hotUpdate` exists.
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
        DEVUP_CSS_FILE_RE.test(fileName) &&
        resolve(importer ? join(dirname(importer), id) : id) ===
          resolve(join(cssDir, fileName))
      ) {
        // Dev re-resolves through a changing id so a growing sheet invalidates
        // the module. A build must not: a per-resolution id forks one file into
        // several modules and makes output hashes differ between identical
        // builds.
        if (!isServe) return join(cssDir, fileName)
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
      if (DEVUP_CSS_FILE_RE.test(fileName)) {
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
        code: extractedCode,
        css = '',
        map,
        cssFile,
        updatedBaseStyle,
        dependencies = [],
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
      for (const dependency of dependencies) this.addWatchFile(dependency)
      const promises: Promise<void>[] = []

      if (updatedBaseStyle) {
        // update base style
        promises.push(writeCssFile('devup-ui.css', getCss(null, false)))
      }

      if (cssFile) {
        const fileNum = getFileNumByFilename(cssFile)
        const prevCss = cssMap.get(fileNum)
        if (prevCss && prevCss.length < css.length) cssMap.set(fileNum, css)
        // `css` is only set when this transform added styles to the sheet.
        if (css) promises.push(writeCssFile(basename(cssFile), css))
      }
      await Promise.all(promises)
      return {
        code: extractedCode,
        map,
      }
    },
    async generateBundle(_options, bundle) {
      if (!extractCss) return
      const writesOutput = this.environment?.config.build?.write !== false
      const cssFiles = new Set<string>()

      // `load` can only snapshot the sheet as it stood when the module was
      // pulled in, and module order varies per build, so the emitted asset was
      // neither complete nor reproducible. Every transform has run by now, so
      // re-read each file's finished sheet instead. Applies to `devup-ui-N.css`
      // too, not just the base sheet.
      for (const file of Object.keys(bundle)) {
        const asset = bundle[file]
        if (!asset.name) continue
        const cssName = getDevupCssChunkName(asset.name)
        if (!cssName) continue
        if (!('source' in asset)) continue
        const source = getCss(getFileNumByFilename(cssName), false)
        asset.source = source
        cssFiles.add(file)
      }

      const environment = this.environment
      if (!environment || !writesOutput) return
      if (environment.config.consumer === 'client' && serverBundleToForward) {
        // @vitejs/plugin-rsc forwards every CSS file referenced by the RSC
        // bundle into the client bundle. Files the client already emitted are
        // registered twice and trigger FILE_NAME_CONFLICT. Keep both bundles'
        // imports and client metadata intact, but remove overlaps from the RSC
        // forwarding set before its later generateBundle hook reads it.
        for (const output of Object.values(serverBundleToForward)) {
          for (const file of cssFiles) {
            output.viteMetadata?.importedCss?.delete(file)
          }
        }
      } else if (environment.config.consumer === 'server') {
        serverBundleToForward = bundle as unknown as Record<
          string,
          ViteOutputWithMetadata
        >
      }
    },
  }
}
