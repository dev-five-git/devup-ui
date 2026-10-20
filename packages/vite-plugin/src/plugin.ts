import { existsSync } from 'node:fs'
import { mkdir, writeFile } from 'node:fs/promises'
import { basename, dirname, join, relative, resolve } from 'node:path'

import {
  beginBuild,
  collectNumberedFiles,
  createCompatTypes,
  createModuleResolver,
  createStateWriter,
  createThemeInterfaceArgs,
  type CustomShorthands,
  extractedNeedles,
  getFileNumByFilename,
  type ImportAliases,
  isMdxSource,
  isSelectedSource,
  loadDevupConfig,
  mergeImportAliases,
  normalizeMdxExtensions,
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
import { createCompiledGuard } from './compiled-guard'
import { createExtractionGate } from './production-manifest'
import { ProductionActivation } from './production-numbering'
import { createMissingInputWatch } from './resolution-watch'

/**
 * CSS entry files emitted by devup-ui: `devup-ui.css`, `devup-ui-3.css`, ...
 *
 * Anchored at BOTH ends and matched against a bare file name. Without `^` it
 * also accepts an app's own `vendor-devup-ui.css`, which `generateBundle` would
 * then overwrite with the devup sheet.
 */
const DEVUP_CSS_FILE_RE = /^devup-ui(-\d+)?\.css$/

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
  root: string,
): Promise<string[]> {
  const config = await loadDevupConfig(options.devupFile)
  const theme = config.theme ?? {}

  registerTheme(withTailwindCss(theme, config, root))
  const tailwindFiles = tailwindCssFiles(config, root)
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
  return tailwindFiles
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
  atomHoist,
  importAliases: userImportAliases,
  mdxExtensions: configuredMdxExtensions,
}: Partial<DevupUIPluginOptions> = {}): PluginOption {
  const mdxExtensions = normalizeMdxExtensions(configuredMdxExtensions)
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
  const importAliases = mergeImportAliases(userImportAliases)
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
  const missingInputWatch = createMissingInputWatch()
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
            prepareSource: (path: string) =>
              production.modules(context).source(path),
          }),
    })
  }
  // The dev server watches cssDir, so every write is an update signal. A
  // module transformed again writes its sheet again, and the reload that
  // signal causes transforms it once more: signal only a changed sheet.
  const writtenCss = new Map<string, string>()
  const stateWriter = createStateWriter((path, content, encoding) =>
    encoding ? writeFile(path, content, encoding) : writeFile(path, content),
  )
  // The files the project's Tailwind CSS is read from, which change what a
  // Tailwind class compiles to
  let tailwindFiles: readonly string[] = []
  const isTailwindFile = (file: string) =>
    tailwindFiles.some(
      (tailwindFile) => resolve(tailwindFile) === resolve(file),
    )
  function writeCssFile(fileName: string, css: string): Promise<void> {
    if (writtenCss.get(fileName) === css) return Promise.resolve()
    writtenCss.set(fileName, css)
    return stateWriter.write(join(cssDir, fileName), css, 'utf-8')
  }
  const sourceTransform = {
    async transform(code, id) {
      if (!extractCss) return
      resolveFallbackPaths()
      const fileName = id.replace(/\?.*$/s, '')
      if (!extractionGate.selected(fileName)) return
      const environment = this.environment ?? plugin
      if (production.active) {
        await production.prepare(observeSetup)
        tailwindFiles = production.tailwindFiles
        for (const path of tailwindFiles)
          this.addWatchFile(path.replaceAll('\\', '/'))
      }
      const context = production.active
        ? (this.environment?.name ?? production.context)
        : undefined
      const preserveSymlinks =
        this.environment?.config.resolve.preserveSymlinks ??
        resolvedConfig?.resolve?.preserveSymlinks
      const watch =
        isServe ||
        this.meta?.watchMode ||
        !!(
          this.environment?.config.build?.watch ?? resolvedConfig?.build?.watch
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
      const environmentConditions = this.environment?.config.resolve.conditions
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
          for (const path of inputs.fileDependencies)
            this.addWatchFile(
              (watch
                ? resolutionWatchPath(path, preserveSymlinks)
                : path
              ).replaceAll('\\', '/'),
            )
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
              ...(isMdxSource(fileName, mdxExtensions)
                ? (['compiled-mdx'] as const)
                : ([] as const)),
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
      } = (() => {
        try {
          return {
            code: output.code,
            css: output.css,
            map: output.map,
            cssFile: output.cssFile,
            updatedBaseStyle: output.updatedBaseStyle,
            dependencies: output.dependencies,
          }
        } finally {
          if ('free' in output) output.free()
        }
      })()
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
      await Promise.all(promises)
      return { code: extractedCode, map }
    },
  } satisfies Pick<Plugin, 'transform'>
  const plugin = {
    name: 'devup-ui',
    // The WASM sheet and transform state are intentionally shared. Vite
    // otherwise recreates this plugin for every environment build, which makes
    // each environment independently emit the same CSS asset.
    sharedDuringBuild: true,
    configureServer(server) {
      missingInputWatch.attach(server)
    },
    async configResolved(config) {
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
        tailwindFiles = await writeDataFiles(
          { package: libPackage, cssDir, devupFile, distDir, singleCss },
          projectRoot,
        )

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
      endBuild()
    },
    async buildStart(options) {
      if (production.active) {
        await production.prepare(observeSetup, () => {
          writtenCss.clear()
          cssMap.clear()
        })
        production.start(this.environment?.name ?? production.context)
        tailwindFiles = production.tailwindFiles
      }
      const watch =
        isServe ||
        this.meta?.watchMode ||
        !!(
          this.environment?.config.build?.watch ?? resolvedConfig?.build?.watch
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
                this.environment?.config.resolve.preserveSymlinks ??
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
    async watchChange(this: void, id) {
      if (production.active) production.invalidate(id.replaceAll('\\', '/'))
      resolveFallbackPaths()
      if (
        (resolve(id) === resolve(devupFile) && existsSync(devupFile)) ||
        (isTailwindFile(id) && existsSync(id))
      ) {
        try {
          if (production.active) {
            await production.updateTheme()
            tailwindFiles = production.tailwindFiles
            return
          }
          tailwindFiles = await writeDataFiles(
            { package: libPackage, cssDir, devupFile, distDir, singleCss },
            projectRoot,
          )
        } catch (error) {
          if (production.active) throw error
          console.error(`[devup-ui] theme update failed at ${devupFile}`, error)
        }
      }
    },
    buildApp: {
      order: 'pre',
      async handler(builder) {
        if (!production.active) return
        production.refine(builder.environments)
        await production.prepare(observeSetup)
      },
    },
    buildEnd: {
      order: 'post',
      async handler(error) {
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
      endBuild()
    },
    // Runs once per environment. Vite 6+ ignores `handleHotUpdate` on a plugin
    // that defines this hook, so the devup.json reload lives here as well.
    async hotUpdate({ file, modules, timestamp }) {
      resolveFallbackPaths()
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
      const isDevupFile =
        resolve(file) === resolve(devupFile) && existsSync(devupFile)
      const isTailwind = isTailwindFile(file) && existsSync(file)
      if (!isDevupFile && !isTailwind) {
        return
      }

      if (production.active) await production.updateTheme()
      else
        tailwindFiles = await writeDataFiles(
          { package: libPackage, cssDir, devupFile, distDir, singleCss },
          projectRoot,
        )

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
      const isDevupFile =
        resolve(file) === resolve(devupFile) && existsSync(devupFile)
      const isTailwind = isTailwindFile(file) && existsSync(file)
      if (!isDevupFile && !isTailwind) {
        return
      }

      if (production.active) await production.updateTheme()
      else
        tailwindFiles = await writeDataFiles(
          { package: libPackage, cssDir, devupFile, distDir, singleCss },
          projectRoot,
        )

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
        const snapshot = () => {
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
    async transform(code, id) {
      if (!isMdxSource(id, mdxExtensions)) return
      return sourceTransform.transform.call(this, code, id)
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
  })
  const plugins: [
    typeof plugin,
    typeof restorePlugin,
    typeof mdxPlugin,
    typeof aggregateGuard,
    typeof compiledGuard,
  ] = [plugin, restorePlugin, mdxPlugin, aggregateGuard, compiledGuard]
  return plugins
}
