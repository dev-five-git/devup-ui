import { existsSync } from 'node:fs'
import { mkdir, writeFile } from 'node:fs/promises'
import { basename, dirname, join, relative, resolve } from 'node:path'

import {
  beginBuild,
  collectNumberedFiles,
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
  seedFileNumbers,
  tailwindCssFiles,
  withTailwindCss,
} from '@devup-ui/plugin-utils'
import {
  codeExtract,
  getCss,
  getDefaultTheme,
  getThemeInterface,
  registerShorthands,
  registerTheme,
  resetBuildState,
  seedFileMap,
  setDebug,
  setModuleResolver,
  setPrefix,
} from '@devup-ui/wasm'
import type {
  EnvironmentModuleNode,
  ModuleNode,
  Plugin,
  PluginOption,
  ResolvedConfig,
  UserConfig,
} from 'vite'

import { createCompiledDependencies } from './compiled-dependencies'
import { createViteReservation } from './reservation'
import { createServerWatch } from './server-watch'

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
  type?: string
  fileName?: string
  viteMetadata?: {
    importedCss?: Set<string>
  }
}

type ForwardableBundle = Record<string, ViteOutputWithMetadata>

/**
 * Name a CSS file is parked under while @vitejs/plugin-rsc forwards it, chosen
 * so that nothing treats it as a stylesheet and it cannot meet a real output.
 */
function getForwardStandInName(file: string): string {
  return `${file}.devup-forwarded`
}

/**
 * @vitejs/plugin-rsc copies every CSS file its server bundle references into
 * the client output with `emitFile`. For a file the client already emitted that
 * is a FILE_NAME_CONFLICT, and the copy replaces the client's own, dropping the
 * names its build manifest is keyed by. The same plugin reads the server
 * bundle's `importedCss` to learn which CSS each server page depends on, so
 * that metadata has to stay whole.
 *
 * It takes what it forwards from `bundle[file]`. For each file the client
 * already owns, the server bundle gets a stand-in copy under another name: the
 * forward lands on a throwaway file and the real one is emitted once.
 *
 * Returns the undo that puts the originals back and drops the stand-ins.
 */
function parkForwardedCss(
  clientBundle: ForwardableBundle,
  serverBundle: ForwardableBundle,
): (outputBundle: ForwardableBundle) => void {
  const files = new Set<string>()
  for (const output of Object.values(serverBundle)) {
    for (const file of output.viteMetadata?.importedCss ?? []) {
      if (file in clientBundle && serverBundle[file]?.type === 'asset') {
        files.add(file)
      }
    }
  }
  for (const file of files) {
    serverBundle[file].fileName = getForwardStandInName(file)
  }
  return (outputBundle) => {
    for (const file of files) {
      serverBundle[file].fileName = file
      delete outputBundle[getForwardStandInName(file)]
    }
  }
}

interface RscPluginApi {
  manager?: { bundles?: Record<string, ForwardableBundle> }
}

/**
 * The server bundles @vitejs/plugin-rsc itself tracks (and later reads to
 * forward CSS). Bundle objects handed to a plugin are per-plugin views, and
 * only the ones plugin-rsc holds reflect an edit it will see.
 */
function getRscServerBundles(config: ResolvedConfig | undefined) {
  const rsc = config?.plugins.find((plugin) => plugin.name === 'rsc:minimal')
  const bundles = (rsc?.api as RscPluginApi | undefined)?.manager?.bundles
  return Object.entries(bundles ?? {})
    .filter(([name]) => name !== 'client')
    .map(([, bundle]) => bundle)
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
   * Reserved for route-aware atom hoisting after PR B adds shared class-slot
   * reservation. Until then, Vite keeps per-file names and warns
   * once for explicit finite positive requests. Ignored in singleCss mode.
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
): Promise<string[]> {
  let tailwindFiles: string[] = []
  try {
    const config = await loadDevupConfig(options.devupFile)
    const theme = config.theme ?? {}

    registerTheme(withTailwindCss(theme, config))
    tailwindFiles = tailwindCssFiles(config)
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
  return tailwindFiles
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
  if (
    !singleCss &&
    atomHoist !== undefined &&
    Number.isFinite(atomHoist) &&
    atomHoist > 0
  ) {
    console.warn(
      '[devup-ui] Vite atomHoist is unavailable until shared class-slot reservation lands in PR B; keeping per-file names, while existing shared slots remain arrival-ordered.',
    )
  }
  registerShorthands(shorthands ?? {})
  setDebug(debug)
  setPrefix(prefix ?? null)
  const importAliases = mergeImportAliases(userImportAliases)
  const reservation = createViteReservation({
    package: libPackage,
    include,
    importAliases,
    singleCss,
    cssDir,
    distDir,
  })
  const cssMap = new Map()
  const compiledDependencies = createCompiledDependencies()
  let stopWatching: (() => void) | undefined
  let watchConfigInputs: (() => void) | undefined
  let resolvedConfig: ResolvedConfig | undefined
  // Set by the client `generateBundle`, run by the late hook of the sibling
  // plugin once @vitejs/plugin-rsc has forwarded.
  let restoreForwardedCss:
    ((outputBundle: ForwardableBundle) => void) | undefined
  let isServe = false
  // The dev server watches cssDir, so every write is an update signal. A
  // module transformed again writes its sheet again, and the reload that
  // signal causes transforms it once more: signal only a changed sheet.
  const writtenCss = new Map<string, string>()
  const stateWriter = createStateWriter((path, content, encoding) =>
    encoding ? writeFile(path, content, encoding) : writeFile(path, content),
  )
  // The files the project's Tailwind CSS is read from, which change what a
  // Tailwind class compiles to
  let tailwindFiles: string[] = []
  const isTailwindFile = (file: string) =>
    tailwindFiles.some(
      (tailwindFile) => resolve(tailwindFile) === resolve(file),
    )
  function writeCssFile(fileName: string, css: string): Promise<void> {
    if (writtenCss.get(fileName) === css) return Promise.resolve()
    writtenCss.set(fileName, css)
    return stateWriter.write(join(cssDir, fileName), css, 'utf-8')
  }
  const plugin: Plugin = {
    name: 'devup-ui',
    // The WASM sheet and transform state are intentionally shared. Vite
    // otherwise recreates this plugin for every environment build, which makes
    // each environment independently emit the same CSS asset.
    sharedDuringBuild: true,
    async configResolved(config) {
      resolvedConfig = config
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
      tailwindFiles = await writeDataFiles({
        package: libPackage,
        cssDir,
        devupFile,
        distDir,
        singleCss,
      })

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
      reservation.configure(config)
    },
    async buildApp(builder) {
      for (const environment of Object.values(builder.environments)) {
        reservation.configure({
          ...builder.config,
          environments: {
            [environment.name]: environment.config,
          },
        })
      }
    },
    configureServer(server) {
      stopWatching?.()
      for (const environment of Object.values(server.environments ?? {})) {
        reservation.configure({
          ...server.config,
          environments: {
            [environment.name]: environment.config,
          },
        })
      }
      const watching = createServerWatch(
        server.watcher,
        reservation,
        compiledDependencies.remove,
      )
      watchConfigInputs = () => watching.add([devupFile, ...tailwindFiles])
      watchConfigInputs()
      stopWatching = () => {
        watching.close()
        watchConfigInputs = undefined
      }
    },
    async buildStart() {
      if (!extractCss) return
      await reservation.reserve()
      for (const dependency of reservation.dependencies)
        this.addWatchFile(dependency)
    },
    config(this: { meta?: ConfigHookMeta } | void, userConfig: UserConfig) {
      const theme = getDefaultTheme()
      const define: Record<string, string> = {}
      if (theme) {
        define['process.env.DEVUP_UI_DEFAULT_THEME'] = JSON.stringify(theme)
      }
      const ret: Omit<UserConfig, 'plugins'> = {
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
      stopWatching?.()
      endBuild()
    },
    async watchChange(id) {
      reservation.invalidate(id)
      if (
        (resolve(id) === resolve(devupFile) && existsSync(devupFile)) ||
        (isTailwindFile(id) && existsSync(id))
      ) {
        try {
          tailwindFiles = await writeDataFiles({
            package: libPackage,
            cssDir,
            devupFile,
            distDir,
            singleCss,
          })
          watchConfigInputs?.()
        } catch (error) {
          console.error(error)
        }
      }
    },
    // Runs once per environment. Vite 6+ ignores `handleHotUpdate` on a plugin
    // that defines this hook, so the devup.json reload lives here as well.
    async hotUpdate({ file, modules, timestamp }) {
      reservation.invalidate(file, timestamp)
      const { environment } = this
      for (const id of compiledDependencies.consumers(environment.name, file)) {
        const consumer = environment.moduleGraph.getModuleById(id)
        if (consumer)
          environment.moduleGraph.invalidateModule(
            consumer,
            new Set(),
            timestamp,
            true,
          )
      }
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
      const isDevupFile =
        resolve(file) === resolve(devupFile) && existsSync(devupFile)
      const isTailwind = isTailwindFile(file) && existsSync(file)
      if (!isDevupFile && !isTailwind) {
        return
      }

      tailwindFiles = await writeDataFiles({
        package: libPackage,
        cssDir,
        devupFile,
        distDir,
        singleCss,
      })
      watchConfigInputs?.()

      const invalidatedModules = new Set<EnvironmentModuleNode>()
      // A Tailwind class compiles from the definitions of the Tailwind CSS, so
      // every module is compiled again
      for (const mod of isTailwind
        ? environment.moduleGraph.idToModuleMap.values()
        : modules) {
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
      reservation.invalidate(file, timestamp)
      for (const id of compiledDependencies.consumers('client', file)) {
        const consumer = server.moduleGraph.getModuleById(id)
        if (consumer)
          server.moduleGraph.invalidateModule(
            consumer,
            new Set(),
            timestamp,
            true,
          )
      }
      const isDevupFile =
        resolve(file) === resolve(devupFile) && existsSync(devupFile)
      const isTailwind = isTailwindFile(file) && existsSync(file)
      if (!isDevupFile && !isTailwind) {
        return
      }

      tailwindFiles = await writeDataFiles({
        package: libPackage,
        cssDir,
        devupFile,
        distDir,
        singleCss,
      })
      watchConfigInputs?.()

      const invalidatedModules = new Set<ModuleNode>()
      for (const mod of isTailwind
        ? server.moduleGraph.idToModuleMap.values()
        : modules) {
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
      await reservation.reserve()

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
      compiledDependencies.update(
        this.environment?.name ?? 'client',
        id,
        dependencies,
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
      if (environment.config.consumer === 'client') {
        const undo = getRscServerBundles(resolvedConfig).map((serverBundle) =>
          parkForwardedCss(
            bundle as unknown as ForwardableBundle,
            serverBundle,
          ),
        )
        restoreForwardedCss = (outputBundle) => {
          for (const restore of undo) restore(outputBundle)
        }
      }
    },
  }
  const restorePlugin: Plugin = {
    name: 'devup-ui:restore-forwarded-css',
    sharedDuringBuild: true,
    apply: 'build',
    generateBundle: {
      order: 'post',
      handler(_options, bundle) {
        restoreForwardedCss?.(bundle as unknown as ForwardableBundle)
        restoreForwardedCss = undefined
      },
    },
  }
  return [plugin, restorePlugin]
}
