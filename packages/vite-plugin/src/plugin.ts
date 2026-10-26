import { existsSync } from 'node:fs'
import { mkdir, writeFile } from 'node:fs/promises'
import { basename, dirname, join, relative, resolve } from 'node:path'

import {
  beginBuild,
  collectNumberedFiles,
  ConfigLoadError,
  createCompatTypes,
  createModuleResolver,
  createStateWriter,
  createThemeInterfaceArgs,
  type CustomShorthands,
  extractedNeedles,
  getBuildOwnership,
  getFileNumByFilename,
  type ImportAliases,
  isMdxSource,
  isSelectedSource,
  loadDevupConfig,
  mergeImportAliases,
  normalizeMdxExtensions,
  readJsxImportSource,
  remapMdxError,
  type ResolutionInputObserver,
  resolutionWatchPath,
  resolveProjectPaths,
  resolveSourceDirs,
  seedFileNumbers,
  SOURCE_FILE_RE,
  tailwindCssFiles,
  withTailwindCss,
} from '@devup-ui/plugin-utils'
import {
  codeExtract,
  getCss as getEngineCss,
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

import { createAggregateCssPreparation } from './aggregate-css'
import { createCompiledDependencies } from './compiled-dependencies'
import { createCompiledGuard } from './compiled-guard'
import { ConfigRefresh, watchConfigFiles } from './config-refresh'
import { DeferredDefaults } from './deferred-defaults'
import { validateMdxDefault } from './mdx-default'
import { createExtractionGate } from './production-manifest'
import { ProductionActivation } from './production-numbering'
import { createViteReservation } from './reservation'
import { createMissingInputWatch } from './resolution-watch'
import { createServerWatch } from './server-watch'

/**
 * CSS entry files emitted by devup-ui: `devup-ui.css`, `devup-ui-3.css`, ...
 *
 * Anchored at BOTH ends and matched against a bare file name. Without `^` it
 * also accepts an app's own `vendor-devup-ui.css`, which `generateBundle` would
 * then overwrite with the devup sheet.
 */
const DEVUP_CSS_FILE_RE = /^devup-ui(-\d+)?\.css$/

type SourceInput = { readonly code: string }
type ExtractionCapture = {
  readonly session: ExtractionSession
  readonly file: string
  readonly input: SourceInput
  readonly ownership: number
  readonly reservation: number
  readonly config: number
}
type ExtractionFlight = {
  readonly capture: ExtractionCapture
  readonly result: { readonly code: string; readonly map: string | undefined }
  readonly dependencies: readonly string[]
  readonly ready: Promise<void>
}
type ExtractionSession = {
  ready: boolean
  readonly flights: Map<string, ExtractionFlight>
}
function createExtractionSession(): ExtractionSession {
  return { ready: false, flights: new Map() }
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
  sourceDirs?: string | string[]
  mdxExtensions?: readonly string[]
  /**
   * Hoist canonical buckets reached by at least this many module boundaries.
   * Defaults to 2; 0 disables. Frozen for this build; ignored in singleCss mode.
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
  root: string,
  inputs: Set<string>,
  publishTailwind: (files: string[]) => void,
): Promise<void> {
  const config = await loadDevupConfig(options.devupFile, inputs)
  const theme = config.theme ?? {}

  registerTheme(withTailwindCss(theme, config, root))
  publishTailwind(tailwindCssFiles(config, root))
  const interfaceCode = getThemeInterface(
    ...createThemeInterfaceArgs(options.package),
  )

  await writeFile(join(options.distDir, 'theme.d.ts'), interfaceCode, 'utf-8')
  // Sequential: writing into cssDir concurrently with its own mkdir loses the
  // race on a cold start (no `df/`) and fails the build with ENOENT.
  if (!existsSync(options.cssDir)) {
    await mkdir(options.cssDir, { recursive: true })
  }
  if (!options.singleCss) {
    await writeFile(
      join(options.cssDir, 'devup-ui.css'),
      getEngineCss(null, false),
    )
  }
}

export function DevupUI({
  package: libPackage = '@devup-ui/react',
  devupFile = 'devup.json',
  distDir = 'df',
  cssDir: configuredCssDir,
  extractCss = true,
  debug = false,
  include = [],
  singleCss = false,
  prefix,
  shorthands,
  sourceDirs: configuredSourceDirs,
  atomHoist = 2,
  importAliases: userImportAliases,
  mdxExtensions: configuredMdxExtensions,
}: Partial<DevupUIPluginOptions> = {}): PluginOption {
  const mdxExtensions = normalizeMdxExtensions(configuredMdxExtensions)
  const deferred = new DeferredDefaults()
  let projectRoot = process.cwd()
  let initialized = false
  let releaseLegacy: (() => void) | undefined
  const integration = {
    integration: 'Vite',
    get root() {
      return projectRoot
    },
  }
  const endBuild = () => {
    releaseLegacy?.()
    releaseLegacy = undefined
  }
  let closing = false
  let ownedWork = 0
  let drained: PromiseWithResolvers<void> | undefined
  function releaseBuild() {
    endBuild()
  }
  function own<This, Args extends unknown[], T>(
    work: (this: This, ...args: Args) => Promise<T>,
    admit = true,
  ) {
    return async function (this: This, ...args: Args): Promise<T | undefined> {
      if (admit && closing && isServe) return
      if (admit && !releaseLegacy && !production.active) {
        releaseLegacy = beginBuild(
          initialized ? {} : { resetBuildState },
          integration,
        )
        initialized = true
        closing = false
        drained = undefined
      }
      ownedWork++
      try {
        return await work.apply(this, args)
      } finally {
        ownedWork--
        if (closing && ownedWork === 0) {
          releaseBuild()
          drained?.resolve()
        }
      }
    }
  }
  const importAliases = mergeImportAliases(
    userImportAliases,
    readJsxImportSource(),
  )
  const production = new ProductionActivation(
    {
      package: libPackage,
      devupFile,
      distDir,
      cssDir: configuredCssDir,
      extractCss,
      debug,
      include,
      singleCss,
      prefix,
      shorthands,
      sourceDirs: configuredSourceDirs,
      mdxExtensions,
      atomHoist,
      importAliases,
    },
    endBuild,
  )
  const getCss = (file: number | null | undefined, main: boolean) =>
    production.active ? production.css(file, main) : getEngineCss(file, main)
  const extractionGate = createExtractionGate({
    package: libPackage,
    include,
    mdxExtensions,
    importAliases,
  })
  const extracted = new Set<string>()
  const pathOptions = { devupFile, distDir, cssDir: configuredCssDir }
  let cssDir = configuredCssDir ?? join(distDir, 'devup-ui')
  let pathsResolved = false
  function resolveFallbackPaths() {
    if (!pathsResolved) {
      ;({ devupFile, distDir, cssDir } = resolveProjectPaths(
        projectRoot,
        pathOptions,
      ))
      pathsResolved = true
    }
  }
  const cssMap = new Map()
  const aggregateCss = createAggregateCssPreparation()
  const reservation = createViteReservation({
    package: libPackage,
    include,
    importAliases,
    singleCss,
    atomHoist,
    get cssDir() {
      return cssDir
    },
    get distDir() {
      return distDir
    },
    relevant: (file) =>
      isConfigFile(file) ||
      isTailwindFile(file) ||
      compiledDependencies.hasSupplier(file),
  })
  const compiledDependencies = createCompiledDependencies()
  let stopWatching: (() => void) | undefined
  let watcherSession: ExtractionSession | undefined
  let watchConfigInputs: (() => void) | undefined
  let resolvedConfig: ResolvedConfig | undefined
  // Set by the client `generateBundle`, run by the late hook of the sibling
  // plugin once @vitejs/plugin-rsc has forwarded.
  let restoreForwardedCss:
    ((outputBundle: ForwardableBundle) => void) | undefined
  let isServe = false
  let isProduction = false
  let seedWarningEmitted = false
  const setupWatchFiles = new Set<string>()
  const missingWatchContexts = new Map<object, (path: string) => void>()
  const missingInputWatch = createMissingInputWatch((importer) => {
    latestInputs.delete(resolve(importer))
  })
  let fallbackConditions: readonly string[] = [
    'import',
    'module',
    'require',
    'node',
  ]
  const observeSetup: ResolutionInputObserver = (inputs) => {
    for (const path of [
      ...inputs.fileDependencies,
      ...inputs.missingDependencies,
    ])
      setupWatchFiles.add(path)
  }
  function moduleResolver(
    conditions: readonly string[],
    onResolutionInputs: ResolutionInputObserver = observeSetup,
    context?: string,
  ) {
    return createModuleResolver({
      cwd: projectRoot,
      includeMdx: mdxExtensions,
      conditions,
      onResolutionInputs,
      toId: (path) => path.replaceAll('\\', '/'),
      ...(context === undefined
        ? {}
        : {
            prepareSource: (path: string) => {
              if (isMdxSource(path, mdxExtensions))
                throw new TypeError(
                  'Markdown values have no synchronous compiler input; move the imported value into JS/TS',
                )
              return production.modules(context).source(path)
            },
          }),
    })
  }
  // The dev server watches cssDir, so every write is an update signal. A
  // module transformed again writes its sheet again, and the reload that
  // signal causes transforms it once more: signal only a changed sheet.
  const writtenCss = new Map<string, string>()
  const sheetRevisions = new Map<string, number>()
  const stateWriter = createStateWriter((path, content, encoding) =>
    encoding ? writeFile(path, content, encoding) : writeFile(path, content),
  )
  // The files the project's Tailwind CSS is read from, which change what a
  // Tailwind class compiles to
  let tailwindFiles: readonly string[] = []
  let configFiles = new Set([resolve(devupFile)])
  let activeConfigFiles = configFiles
  let activeTailwindFiles: string[] = []
  const configState = new ConfigRefresh()
  let extractionSession = createExtractionSession()
  const latestInputs = new Map<string, SourceInput>()
  function rotateExtractionSession() {
    latestInputs.clear()
    return (extractionSession = createExtractionSession())
  }
  function currentCapture(capture: ExtractionCapture) {
    const ownership = getBuildOwnership()
    return (
      isServe &&
      !!releaseLegacy &&
      !closing &&
      capture.session === extractionSession &&
      extractionSession.ready &&
      latestInputs.get(capture.file) === capture.input &&
      ownership.running === 1 &&
      ownership.generation === capture.ownership &&
      configState.refreshing === 0 &&
      configState.generation === capture.config &&
      reservation.revision === capture.reservation &&
      reservation.prepared === capture.reservation
    )
  }
  const isConfigFile = (file: string) =>
    configFiles.has(resolve(file)) ||
    activeConfigFiles.has(resolve(file)) ||
    production.configFiles.includes(resolve(file))
  const refreshConfig = () =>
    configState.run(async () => {
      if (production.active) {
        await production.updateTheme(observeSetup)
        tailwindFiles = production.tailwindFiles
        return
      }
      const inputs = new Set([resolve(devupFile)])
      try {
        activeConfigFiles = inputs
        await writeDataFiles(
          {
            package: libPackage,
            cssDir,
            devupFile,
            distDir,
            singleCss,
          },
          projectRoot,
          inputs,
          (files) => {
            activeTailwindFiles = files
            watchConfigInputs?.()
          },
        )
      } finally {
        configFiles = inputs
        tailwindFiles = activeTailwindFiles
        activeTailwindFiles = []
        reservation.invalidate()
        watchConfigInputs?.()
      }
    })
  const isTailwindFile = (file: string) =>
    [...tailwindFiles, ...activeTailwindFiles].some(
      (tailwindFile) => resolve(tailwindFile) === resolve(file),
    )
  function writeCssFile(fileName: string, css: string): Promise<void> {
    if (writtenCss.get(fileName) === css) return Promise.resolve()
    writtenCss.set(fileName, css)
    sheetRevisions.set(fileName, (sheetRevisions.get(fileName) ?? 0) + 1)
    return stateWriter.write(join(cssDir, fileName), css, 'utf-8')
  }
  const sourceTransform = {
    async transform(code, id) {
      const session = extractionSession
      if (!extractCss) return
      resolveFallbackPaths()
      const fileName = id.replace(/\?.*$/s, '')
      if (!extractionGate.selected(fileName)) return
      const beforeRefresh = configState.wait()
      if (beforeRefresh) await beforeRefresh
      const environment = this.environment ?? plugin
      if (production.active) {
        await production.prepare(observeSetup)
        tailwindFiles = production.tailwindFiles
        for (const path of tailwindFiles)
          this.addWatchFile(path.replaceAll('\\', '/'))
      }
      if (!production.active) await reservation.reserve()
      const afterRefresh = configState.wait()
      if (afterRefresh) await afterRefresh
      const file = resolve(fileName)
      let input = latestInputs.get(file)
      if (isServe && input?.code !== code) {
        input = { code }
        latestInputs.set(file, input)
      }
      const capture = input && {
        session,
        file,
        input,
        ownership: getBuildOwnership().generation,
        reservation: reservation.revision,
        config: configState.generation,
      }
      const eligible = capture && currentCapture(capture)
      const cached = eligible ? session.flights.get(id) : undefined
      if (cached && currentCapture(cached.capture)) {
        compiledDependencies.update(
          this.environment?.name ?? 'client',
          id,
          cached.dependencies,
        )
        for (const dependency of cached.dependencies)
          this.addWatchFile(dependency)
        await cached.ready
        await configState.ready()
        return cached.result
      }
      const context = production.active
        ? (this.environment?.name ?? production.context)
        : undefined
      const preserveSymlinks =
        this.environment?.config?.resolve?.preserveSymlinks ??
        resolvedConfig?.resolve?.preserveSymlinks
      const watch =
        isServe ||
        this.meta?.watchMode ||
        !!(
          this.environment?.config?.build?.watch ?? resolvedConfig?.build?.watch
        )
      missingInputWatch.start(fileName, environment)
      aggregateCss.observe(this.environment ?? plugin, id)
      if (
        production.active &&
        !isMdxSource(fileName, mdxExtensions) &&
        !extracted.has(fileName) &&
        !extractionGate.relevant(code) &&
        !extractionGate.admission(code)
      )
        return
      const environmentConditions =
        this.environment?.config?.resolve?.conditions
      const resolver = moduleResolver(
        (environmentConditions
          ? ['import', ...environmentConditions]
          : fallbackConditions
        ).map((condition) =>
          condition === 'development|production'
            ? isProduction
              ? 'production'
              : 'development'
            : condition,
        ),
        (inputs) => {
          for (const path of inputs.fileDependencies) {
            const watched = (
              watch ? resolutionWatchPath(path, preserveSymlinks) : path
            ).replaceAll('\\', '/')
            this.addWatchFile(watched)
            missingInputWatch.observeFile(fileName, watched, environment)
          }
          for (const path of inputs.missingDependencies) {
            const watched = (
              watch ? resolutionWatchPath(path, preserveSymlinks) : path
            ).replaceAll('\\', '/')
            setupWatchFiles.add(path)
            missingInputWatch.observe(fileName, watched, environment)
            missingWatchContexts.get(this.environment ?? plugin)?.(watched)
          }
        },
        context,
      )
      if (!production.active) setModuleResolver(resolver)
      let rel = relative(dirname(id), cssDir).replaceAll('\\', '/')
      if (!rel.startsWith('./')) rel = `./${rel}`
      extracted.add(fileName)
      const output = (() => {
        try {
          const sourceType = isMdxSource(fileName, mdxExtensions)
            ? 'compiled-mdx'
            : undefined
          const sourcePolicy: [
            sourceType?: 'compiled-mdx',
            deferredExtensions?: readonly string[],
          ] = production.active
            ? [sourceType, mdxExtensions]
            : sourceType === undefined
              ? []
              : [sourceType]
          const extract = () =>
            codeExtract(
              fileName,
              code,
              libPackage,
              rel,
              singleCss,
              true,
              false,
              importAliases,
              ...sourcePolicy,
            )
          if (!production.active) return extract()
          const key = context ?? production.context
          return production.run(key, () =>
            production.modules(key).extract(
              {
                nativeId: id,
                id: fileName,
                classify: (id, source = code) => {
                  extracted.add(id)
                  return extractionGate.relevant(source)
                },
                source: {
                  code,
                  ...(isMdxSource(fileName, mdxExtensions)
                    ? {
                        sourceType: 'compiled-mdx' as const,
                        map: this.getCombinedSourcemap(),
                      }
                    : {}),
                },
              },
              resolver,
              extract,
            ),
          )
        } catch (error) {
          if (isMdxSource(fileName, mdxExtensions))
            throw remapMdxError(error, fileName, this.getCombinedSourcemap())
          throw error
        }
      })()
      const {
        code: extractedCode,
        css = '',
        map,
        cssFile,
        updatedBaseStyle,
        dependencies = [],
        deferredDefaults: deferredMetadata,
      } = (() => {
        try {
          return {
            code: output.code,
            css: output.css,
            map: output.map,
            cssFile: output.cssFile,
            updatedBaseStyle: output.updatedBaseStyle,
            dependencies: output.dependencies,
            deferredDefaults: output.deferredDefaults,
          }
        } finally {
          if ('free' in output) output.free()
        }
      })()
      compiledDependencies.update(
        this.environment?.name ?? 'client',
        id,
        dependencies,
      )
      for (const dependency of dependencies) this.addWatchFile(dependency)
      const promises: Promise<void>[] = []
      if (updatedBaseStyle)
        promises.push(writeCssFile('devup-ui.css', getCss(null, false)))
      if (cssFile) {
        const fileNum = getFileNumByFilename(cssFile)
        const prevCss = cssMap.get(fileNum)
        if (prevCss && prevCss.length < css.length) cssMap.set(fileNum, css)
        if (css) promises.push(writeCssFile(basename(cssFile), css))
      }
      const result = {
        code: extractedCode,
        map,
        ...(production.active
          ? { meta: { devupDeferredDefaults: deferredMetadata } }
          : {}),
      }
      if (!eligible) {
        await Promise.all(promises)
        await configState.ready()
        if (production.active)
          deferred.extracted(
            this.environment ?? deferred.fallback,
            id,
            deferredMetadata,
            isMdxSource(fileName, mdxExtensions)
              ? (prefix) =>
                  remapMdxError(prefix, fileName, this.getCombinedSourcemap())
                    .message
              : undefined,
          )
        return result
      }
      const flight: ExtractionFlight = {
        capture,
        result,
        dependencies: [...dependencies],
        ready: Promise.all(promises).then(
          () => {
            if (session.flights.get(id) === flight && !currentCapture(capture))
              session.flights.delete(id)
          },
          (error: unknown) => {
            if (session.flights.get(id) === flight) session.flights.delete(id)
            throw error
          },
        ),
      }
      session.flights.set(id, flight)
      await flight.ready
      await configState.ready()
      return result
    },
  } satisfies Pick<Plugin, 'transform'>
  const plugin = {
    name: 'devup-ui',
    // The WASM sheet and transform state are intentionally shared. Vite
    // otherwise recreates this plugin for every environment build, which makes
    // each environment independently emit the same CSS asset.
    sharedDuringBuild: true,
    async configResolved(config) {
      closing = false
      drained = undefined
      const session = rotateExtractionSession()
      resolvedConfig = config
      try {
        isServe = config?.command === 'serve'
        isProduction = config?.isProduction ?? false
        projectRoot = config?.root ?? process.cwd()
        ;({ devupFile, distDir, cssDir } = resolveProjectPaths(
          projectRoot,
          pathOptions,
        ))
        pathsResolved = true
        const conditions = [
          'import',
          ...(config?.build?.ssr
            ? (config.ssr?.resolve?.conditions ?? [
                'module',
                'node',
                'development|production',
              ])
            : (config?.resolve?.conditions ?? [
                'module',
                'browser',
                'development|production',
              ])),
        ].map((condition) =>
          condition === 'development|production'
            ? config?.isProduction
              ? 'production'
              : 'development'
            : condition,
        )
        // Vite ids are POSIX absolute paths
        fallbackConditions = conditions
        if (production.configure(config)) writtenCss.clear()
        if (production.active) {
          endBuild()
          initialized = true
          return
        }
        if (!releaseLegacy) {
          releaseLegacy = beginBuild(
            initialized ? {} : { resetBuildState },
            integration,
          )
          initialized = true
        }
        registerShorthands(shorthands ?? {})
        setDebug(debug)
        setPrefix(prefix ?? null)
        setModuleResolver(moduleResolver(conditions))
        const sourceDirs = resolveSourceDirs(projectRoot, configuredSourceDirs)
        const input =
          config?.build?.rolldownOptions?.input ??
          config?.build?.rollupOptions?.input ??
          (config?.build?.lib && config.build.lib.entry)
        const rawEntries =
          typeof input === 'string'
            ? [input]
            : Array.isArray(input)
              ? input
              : input && typeof input === 'object'
                ? Object.values(input)
                : []
        const entries = rawEntries
          .filter((entry): entry is string => typeof entry === 'string')
          .filter((entry) => isSelectedSource(entry, mdxExtensions))
          .map((entry) => resolve(projectRoot, entry))
        const roots = [
          ...new Set([
            ...sourceDirs,
            ...entries.map((entry) => dirname(entry)),
          ]),
        ]
        if (!existsSync(distDir)) await mkdir(distDir, { recursive: true })
        await writeFile(join(distDir, '.gitignore'), '*', 'utf-8')
        await writeFile(
          join(distDir, 'compat.d.ts'),
          createCompatTypes(importAliases),
          'utf-8',
        )
        await refreshConfig()

        try {
          // Numbers come from the sorted paths of every file the build can
          // extract (source and included packages), not from arrival order.
          // Files numbered before keep their numbers, so a later pass in the
          // dev server only numbers new files after the existing ones.
          seedFileNumbers(
            { seedFileMap },
            collectNumberedFiles({
              roots,
              includeMdx: mdxExtensions,
              include,
              cwd: projectRoot,
              needles: extractedNeedles(libPackage, importAliases),
              importAliases,
            }),
          )
        } catch (cause) {
          if (!seedWarningEmitted) {
            seedWarningEmitted = true
            console.warn(
              '[devup-ui] deterministic file seeding failed; class IDs now depend on module arrival order',
              { phase: 'seed', root: projectRoot, cause },
            )
          }
        }
      } catch (cause) {
        endBuild()
        throw new Error(
          `[devup-ui] setup failed at ${projectRoot}: ${cause instanceof Error ? cause.message : String(cause)}`,
          { cause },
        )
      }
      reservation.configure(config)
      if (extractionSession === session) session.ready = true
    },
    configureServer(server) {
      missingInputWatch.attach(server)
      if (watcherSession === extractionSession) rotateExtractionSession()
      stopWatching?.()
      watcherSession = extractionSession
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
      watchConfigInputs = () =>
        watching.add([
          ...configFiles,
          ...activeConfigFiles,
          ...tailwindFiles,
          ...activeTailwindFiles,
        ])
      watchConfigInputs()
      stopWatching = () => {
        watching.close()
        watchConfigInputs = undefined
      }
    },
    config(this: { meta?: ConfigHookMeta } | void, userConfig: UserConfig) {
      const release = beginBuild(
        initialized ? {} : { resetBuildState },
        integration,
      )
      initialized = true
      let theme: ReturnType<typeof getDefaultTheme>
      try {
        theme = getDefaultTheme()
      } finally {
        release()
      }
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
      if (
        extractCss &&
        !userConfig?.build?.lib &&
        userConfig?.build?.cssCodeSplit !== false
      ) {
        ret.build = createCssChunkBuildOptions(this?.meta, userConfig)
      }
      return ret
    },
    apply() {
      return true
    },
    closeBundle(this: (object & { environment?: { name: string } }) | void) {
      if (production.active)
        production.close(this ? this.environment?.name : undefined)
      else {
        missingInputWatch.close()
        missingWatchContexts.clear()
      }
      rotateExtractionSession()
      closing = true
      stopWatching?.()
      if (ownedWork === 0) return releaseBuild()
      drained ??= Promise.withResolvers<void>()
      return drained.promise
    },
    async buildStart(options) {
      if (!extractCss) return
      if (production.active) {
        const attempted = new Set<string>()
        try {
          await production.prepare(
            (inputs) => {
              observeSetup(inputs)
              for (const path of [
                ...inputs.fileDependencies,
                ...inputs.missingDependencies,
              ])
                attempted.add(path)
            },
            () => {
              writtenCss.clear()
              cssMap.clear()
            },
          )
        } catch (error) {
          if (error instanceof ConfigLoadError)
            watchConfigFiles([...attempted], (path) => this.addWatchFile(path))
          throw error
        }
        production.start(this.environment?.name ?? production.context)
        tailwindFiles = production.tailwindFiles
        watchConfigFiles(production.configFiles, (path) =>
          this.addWatchFile(path),
        )
      } else {
        await reservation.reserve()
        for (const dependency of reservation.dependencies)
          this.addWatchFile(dependency)
      }
      const watch =
        isServe ||
        this.meta?.watchMode ||
        !!(
          this.environment?.config?.build?.watch ?? resolvedConfig?.build?.watch
        )
      const addWatchFile = (path: string) => {
        this.addWatchFile(path)
        if (isServe || !watch || existsSync(path) || !SOURCE_FILE_RE.test(path))
          return
        let ancestor = dirname(path)
        while (!existsSync(ancestor) && dirname(ancestor) !== ancestor)
          ancestor = dirname(ancestor)
        const physical = resolve(resolutionWatchPath(ancestor, false))
        if (
          dirname(physical) !== physical &&
          physical !== resolve(resolutionWatchPath(projectRoot, false)) &&
          physical !==
            resolve(
              resolutionWatchPath(join(projectRoot, 'node_modules'), false),
            ) &&
          basename(physical) !== 'node_modules'
        )
          this.addWatchFile(physical.replaceAll('\\', '/'))
      }
      missingWatchContexts.set(this.environment ?? plugin, addWatchFile)
      for (const path of setupWatchFiles)
        addWatchFile(
          (watch
            ? resolutionWatchPath(
                path,
                this.environment?.config?.resolve?.preserveSymlinks ??
                  resolvedConfig?.resolve?.preserveSymlinks,
              )
            : path
          ).replaceAll('\\', '/'),
        )
      for (const path of tailwindFiles)
        this.addWatchFile(path.replaceAll('\\', '/'))
      if (isServe || !extractCss) return
      aggregateCss.start(
        this.environment ?? plugin,
        options.input,
        this.environment?.config.build ?? resolvedConfig?.build ?? {},
      )
    },
    async watchChange(id) {
      if (production.active) production.invalidate(id.replaceAll('\\', '/'))
      resolveFallbackPaths()
      reservation.invalidate(id)
      if (isConfigFile(id) || (isTailwindFile(id) && existsSync(id))) {
        try {
          await own(refreshConfig)()
        } finally {
          if (production.active) {
            watchConfigFiles(production.configFiles, (path) =>
              this.addWatchFile(path),
            )
          }
        }
      }
    },
    buildApp: {
      order: 'pre',
      async handler(builder) {
        for (const environment of Object.values(builder.environments)) {
          reservation.configure({
            ...builder.config,
            environments: { [environment.name]: environment.config },
          })
        }
        if (!production.active) return
        production.refine(builder.environments)
        await production.prepare(observeSetup)
      },
    },
    buildEnd: {
      order: 'post',
      async handler(error) {
        if (production.active && !error) {
          try {
            deferred.verify(this.environment ?? deferred.fallback, this)
          } catch (cause) {
            if (cause instanceof Error)
              await production.finish(
                this.environment?.name ?? production.context,
                this,
                cause,
              )
            throw cause
          }
        }
        if (production.active)
          await production.finish(
            this.environment?.name ?? production.context,
            this,
            error,
          )
      },
    },
    closeWatcher() {
      if (production.active) production.dispose()
      missingInputWatch.close()
      missingWatchContexts.clear()
      rotateExtractionSession()
      closing = true
      stopWatching?.()
      if (ownedWork === 0) return releaseBuild()
      drained ??= Promise.withResolvers<void>()
      return drained.promise
    },
    // Runs once per environment. Vite 6+ ignores `handleHotUpdate` on a plugin
    // that defines this hook, so the devup.json reload lives here as well.
    async hotUpdate({ file, modules, timestamp }) {
      resolveFallbackPaths()
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
      const isDevupFile = isConfigFile(file)
      const isTailwind = isTailwindFile(file) && existsSync(file)
      if (!isDevupFile && !isTailwind) {
        return
      }

      await own(refreshConfig)()

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
      resolveFallbackPaths()
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
      const isDevupFile = isConfigFile(file)
      const isTailwind = isTailwindFile(file) && existsSync(file)
      if (!isDevupFile && !isTailwind) {
        return
      }

      await own(refreshConfig)()

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
      resolveFallbackPaths()
      const fileName = basename(id).split('?')[0]
      if (
        DEVUP_CSS_FILE_RE.test(fileName) &&
        resolve(importer ? join(dirname(importer), id) : id) ===
          resolve(join(cssDir, fileName))
      ) {
        if (!isServe) return join(cssDir, fileName)
        // Only changed writes mint a module; `v` survives Vite's timestamp stripping.
        return join(
          cssDir,
          `${fileName}?v=${sheetRevisions.get(fileName) ?? 0}`,
        )
      }
    },
    load(id) {
      const fileName = basename(id).split('?')[0]
      if (DEVUP_CSS_FILE_RE.test(fileName)) {
        const snapshot = (): string | Promise<string> => {
          if (configState.refreshing) return configState.ready().then(snapshot)
          configState.assertReady()
          const fileNum = getFileNumByFilename(fileName)
          const css = getCss(fileNum, false)
          cssMap.set(fileNum, css)
          return css
        }
        const preparation = aggregateCss.prepare(
          this.environment ?? plugin,
          this,
          id,
        )
        return preparation ? preparation.then(snapshot) : snapshot()
      }
    },
    enforce: 'pre',
    transform: {
      async handler(code, id) {
        if (!SOURCE_FILE_RE.test(id.split('?')[0])) return
        return sourceTransform.transform.call(this, code, id)
      },
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
  } satisfies Plugin
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
  const mdxPlugin = {
    name: 'devup-ui:mdx',
    enforce: 'post',
    sharedDuringBuild: true,
    ...deferred.resolution.companion(),
    async transform(code, id) {
      if (!isMdxSource(id, mdxExtensions)) return
      if (production.active) validateMdxDefault(this.parse(code), code, id)
      const result = await sourceTransform.transform.call(this, code, id)
      if (production.active && extractCss && result)
        deferred.resolution.succeeded(this.environment ?? deferred.fallback, id)
      return result
    },
  } satisfies Plugin
  const aggregateGuard = {
    name: 'devup-ui:aggregate-css-guard',
    sharedDuringBuild: true,
    apply: 'build',
    generateBundle: {
      order: 'post',
      handler(_options, bundle) {
        for (const asset of Object.values(bundle)) {
          if (asset.type === 'asset') {
            aggregateCss.checkAsset(this.environment ?? plugin, asset)
          }
        }
      },
    },
  } satisfies Plugin
  const compiledGuard = createCompiledGuard({
    package: libPackage,
    mdxExtensions,
    importAliases,
    extractCss,
    deferred,
  })
  const plugins: [
    typeof plugin,
    typeof restorePlugin,
    typeof mdxPlugin,
    typeof aggregateGuard,
    typeof compiledGuard,
  ] = [plugin, restorePlugin, mdxPlugin, aggregateGuard, compiledGuard]
  plugin.configResolved = own(plugin.configResolved, false)
  plugin.buildStart = own(plugin.buildStart)
  plugin.transform.handler = own(plugin.transform.handler)
  mdxPlugin.transform = own(mdxPlugin.transform)
  return plugins
}
