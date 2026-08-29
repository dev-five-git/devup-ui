import {
  existsSync,
  mkdirSync,
  readFileSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs'
import { dirname, join, relative, resolve } from 'node:path'
import { deserialize, serialize } from 'node:v8'

import {
  buildCanonicalMap,
  buildStaticImportGraph,
  computeCompiledFiles,
  computeFileRoutes,
  createNodeModulesExcludeRegex,
  createThemeInterfaceArgs,
  type DevupUIBasePluginOptions,
  loadDevupConfigSync,
  mergeImportAliases,
  planAtomHoist,
  type StaticImportGraph,
} from '@devup-ui/plugin-utils'
import { type NextConfig } from 'next'

import {
  type PrewarmedOutput,
  startCoordinator,
  takeExtractOutput,
} from './coordinator'
import { collectProductionPrewarmFiles } from './prewarm'
import { elapsedMs, profileStart, reportProfile } from './profile'
import { transformStaticVanillaExtract } from './static-vanilla'
import { loadWasm, loadWebpackPlugin } from './wasm'

/** Options accepted by the Next.js integration. */
export type DevupUINextPluginOptions = Partial<DevupUIBasePluginOptions> & {
  /** Share atoms reached by at least this many routes. */
  atomHoist?: number
}

type TurboRules = NonNullable<NonNullable<NextConfig['turbopack']>['rules']>

interface ProductionTurboSetupCache {
  defaultTheme?: string
  key: string
  owner: string
  prewarmedFiles: number
  rules: TurboRules
  token: string
  wasmVariant: 'lite' | 'full'
}

const productionTurboSetupTokenEnv = 'DEVUP_UI_TURBO_SETUP_TOKEN'
let productionTurboSetupOwner = `${performance.timeOrigin}-${Math.random()}`

function consumeProductionTurboSetup(
  key: string,
  filename: string,
): ProductionTurboSetupCache | undefined {
  const token = process.env[productionTurboSetupTokenEnv]
  if (!token) return undefined
  try {
    const cached = deserialize(readFileSync(filename)) as
      ProductionTurboSetupCache | undefined
    if (
      !cached ||
      cached.key !== key ||
      cached.owner === productionTurboSetupOwner ||
      cached.token !== token
    ) {
      return undefined
    }
    delete process.env[productionTurboSetupTokenEnv]
    try {
      unlinkSync(filename)
    } catch {
      // The handoff is single-use even if cleanup fails.
    }
    return cached
  } catch {
    return undefined
  }
}

function storeProductionTurboSetup(
  filename: string,
  cache: Omit<ProductionTurboSetupCache, 'owner' | 'token'>,
): void {
  const token = `${process.pid}-${performance.now()}-${Math.random()}`
  try {
    writeFileSync(
      filename,
      serialize({
        ...cache,
        owner: productionTurboSetupOwner,
        token,
      } satisfies ProductionTurboSetupCache),
    )
    process.env[productionTurboSetupTokenEnv] = token
  } catch {
    delete process.env[productionTurboSetupTokenEnv]
  }
}

/** @internal Reproduce Next's isolated config-module reload in unit tests. */
export function reloadTurboSetupModuleForTesting(): void {
  productionTurboSetupOwner = `${performance.timeOrigin}-${Math.random()}`
}

/** @internal Keep the process-global one-build handoff isolated between tests. */
export function resetTurboSetupCacheForTesting(): void {
  delete process.env[productionTurboSetupTokenEnv]
  productionTurboSetupOwner = `${performance.timeOrigin}-${Math.random()}`
}

export function selectWasmVariant(
  graph: StaticImportGraph | undefined,
  candidateFiles: string[] = graph?.files ?? [],
  staticVanillaExtract = false,
): 'lite' | 'full' {
  return graph &&
    (staticVanillaExtract ||
      !candidateFiles.some((filename) => /\.css\.(?:ts|js)$/.test(filename)))
    ? 'lite'
    : 'full'
}

/**
 * Devup UI Next Plugin
 * @param config
 * @param options
 * @constructor
 */
export function DevupUI(
  config: NextConfig,
  options: DevupUINextPluginOptions = {},
): NextConfig {
  const pluginStartedAt = profileStart()
  const isTurbo =
    process.env.TURBOPACK === '1' || process.env.TURBOPACK === 'auto'
  // turbopack is now stable, TURBOPACK is set to auto without any flags
  if (isTurbo) {
    config ??= {}
    config.turbopack ??= {}
    config.turbopack.rules ??= {}
    const {
      package: libPackage = '@devup-ui/react',
      distDir = 'df',
      cssDir = resolve(distDir, 'devup-ui'),
      singleCss = false,
      devupFile = 'devup.json',
      include = [],
      prefix,
      shorthands,
      atomHoist,
      importAliases: userImportAliases,
    } = options

    const importAliases = mergeImportAliases(userImportAliases)
    const watch = process.env.NODE_ENV === 'development'
    const sourceMap = watch || config.productionBrowserSourceMaps === true
    const sheetFile = join(distDir, 'sheet.json')
    const classMapFile = join(distDir, 'classMap.json')
    const fileMapFile = join(distDir, 'fileMap.json')
    const canonicalMapFile = join(distDir, 'canonicalMap.json')
    const gitignoreFile = join(distDir, '.gitignore')
    const setupHandoffFile = join(distDir, 'setup.bin')
    // Next loads next.config twice for a production Turbopack build in isolated
    // module contexts, but both evaluations share the same process. Hand the
    // completed first setup to exactly one different module instance so the
    // second load does not rescan/reparse sources, re-extract them, or restart
    // the coordinator. Calls from the same module instance never reuse it,
    // which avoids turning this into a general cross-build cache.
    const productionSetupKey = JSON.stringify({
      atomHoist,
      cwd: process.cwd(),
      cssDir,
      devupFile,
      distDir,
      hoistV: process.env.DEVUP_HOIST_V,
      importAliases,
      include,
      libPackage,
      prefix,
      shorthands,
      singleCss,
      sourceMap,
    })
    const cachedSetup = watch
      ? undefined
      : consumeProductionTurboSetup(productionSetupKey, setupHandoffFile)
    if (cachedSetup) {
      if (cachedSetup.defaultTheme) {
        process.env.DEVUP_UI_DEFAULT_THEME = cachedSetup.defaultTheme
        config.env ??= {}
        Object.assign(config.env, {
          DEVUP_UI_DEFAULT_THEME: cachedSetup.defaultTheme,
        })
      }
      Object.assign(config.turbopack.rules, cachedSetup.rules)
      reportProfile('next.setup', {
        cacheHit: true,
        durationMs: elapsedMs(pluginStartedAt),
        pid: process.pid,
        prewarmedFiles: cachedSetup.prewarmedFiles,
        singleCss,
        wasmVariant: cachedSetup.wasmVariant,
        watch,
      })
      return config
    }
    if (!existsSync(distDir))
      mkdirSync(distDir, {
        recursive: true,
      })
    if (!existsSync(cssDir))
      mkdirSync(cssDir, {
        recursive: true,
      })
    if (!existsSync(gitignoreFile)) writeFileSync(gitignoreFile, '*')

    // Boa is only needed to execute vanilla-extract-style `.css.ts`/`.css.js`
    // modules. Build the graph before touching WASM so ordinary applications
    // instantiate the much smaller engine, while vanilla-extract users retain
    // the full evaluator automatically. If graph discovery fails, fail safe to
    // the full engine.
    const graphStartedAt = profileStart()
    const srcDir = resolve(process.cwd(), 'src')
    const tsconfigPath = resolve(process.cwd(), 'tsconfig.json')
    let staticGraph: StaticImportGraph | undefined
    try {
      staticGraph = buildStaticImportGraph(srcDir, tsconfigPath)
    } catch {
      // The mapping pass below reports the graph failure and keeps its legacy
      // best-effort behavior.
    }
    const candidateCollectStartedAt =
      graphStartedAt === undefined ? undefined : performance.now()
    const wasmCandidateFiles = staticGraph
      ? collectProductionPrewarmFiles({
          cwd: process.cwd(),
          graph: staticGraph,
          expectedBaseFiles: [],
          libPackage,
          include,
        })
      : []
    const candidateCollectMs = elapsedMs(candidateCollectStartedAt)
    const staticVanillaSources = new Map<
      string,
      { code: string; source: string }
    >()
    const vanillaCandidates = wasmCandidateFiles.filter((filename) =>
      /\.css\.(?:ts|js)$/.test(filename),
    )
    let staticVanillaExtract =
      !watch && !sourceMap && vanillaCandidates.length > 0
    for (const filename of vanillaCandidates) {
      if (!staticVanillaExtract) break
      const source = readFileSync(resolve(process.cwd(), filename), 'utf-8')
      const code = transformStaticVanillaExtract(filename, source, libPackage)
      if (code === undefined) {
        staticVanillaExtract = false
        staticVanillaSources.clear()
        break
      }
      staticVanillaSources.set(filename, { code, source })
    }
    const wasmVariant = selectWasmVariant(
      staticGraph,
      wasmCandidateFiles,
      staticVanillaExtract,
    )
    const wasm = loadWasm(wasmVariant === 'lite')
    const {
      codeExtract,
      codeExtractWithoutSourceMap,
      exportClassMap,
      exportFileMap,
      exportSheet,
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
      setAtomHoist,
      setPrefix,
    } = wasm

    registerShorthands(shorthands ?? {})

    if (prefix) {
      setPrefix(prefix)
    }

    // Import previous session state to handle Turbopack persistent cache.
    // When the dev server restarts, Turbopack may skip re-running loaders for
    // unchanged files. Without importing previous state, the coordinator's WASM
    // starts empty and CSS for cached files would be missing.
    try {
      importSheet(JSON.parse(readFileSync(sheetFile, 'utf-8')))
      importClassMap(JSON.parse(readFileSync(classMapFile, 'utf-8')))
      importFileMap(JSON.parse(readFileSync(fileMapFile, 'utf-8')))
    } catch {
      // No previous session state (first run) or corrupt files — start fresh
    }

    const devupConfig = loadDevupConfigSync(devupFile)

    const theme: any = devupConfig.theme ?? {}
    // Register current theme after importing previous state,
    // since importSheet replaces the entire sheet including its theme.
    registerTheme(theme)
    const themeInterface = getThemeInterface(
      ...createThemeInterfaceArgs(libPackage),
    )
    if (themeInterface) {
      writeFileSync(join(distDir, 'theme.d.ts'), themeInterface)
    }
    // disable turbo parallel
    const excludeRegex = createNodeModulesExcludeRegex(include, '.mdx.[tj]sx?$')

    const coordinatorPortFile = join(distDir, 'coordinator.port')

    // Pre-pass: single-importer collapse ALWAYS runs (files with exactly one
    // importer merge into that importer's bucket, so their identical atoms share
    // one class). Atom-level hoisting COMPOSES on top: an atom reached by
    // >= atomHoist distinct routes is emitted once into the shared devup-ui.css.
    //
    // The two compose because both are keyed by the canonical bucket: the engine
    // keys property buckets by canonical(filename), and the route-reach map below
    // is folded onto the SAME canonical bucket — so route_count_for_files() looks
    // atoms up by bucket and the lookup hits. `atomHoist` must be configured
    // BEFORE any extraction so atoms receive global (shared) class names; the
    // coordinator shares this WASM instance, so it applies to every /extract.
    const atomMode =
      atomHoist !== undefined && Number.isFinite(atomHoist) && atomHoist > 0
    const extract = sourceMap ? codeExtract : codeExtractWithoutSourceMap
    // Hoisted out of the try so the coordinator can receive it for per-bucket
    // completion. Stays `{}` if the best-effort pre-pass fails.
    let canonicalMap: Record<string, string> = {}
    // Every runtime file the bundler will compile (cwd-relative POSIX) — the
    // deterministic base-css completion signal handed to the coordinator. Stays
    // `[]` (idle fallback) when no routes are detected or the pre-pass fails.
    let expectedBaseFiles: string[] = []
    try {
      if (!staticGraph) throw new Error('Static import graph unavailable')
      const cwd = process.cwd()
      // One scan+parse of the source tree, shared by all three consumers below.
      const graph = staticGraph
      // Atom hoisting owns the shared-chunk decision, so collapse runs WITHOUT
      // the file-level @global hoist (DEVUP_HOIST_V) in atom mode.
      const hoistV = atomMode
        ? undefined
        : process.env.DEVUP_HOIST_V
          ? Number(process.env.DEVUP_HOIST_V)
          : undefined
      canonicalMap = buildCanonicalMap({
        srcDir,
        tsconfigPath,
        cwd,
        hoistV,
        graph,
      })
      importCanonicalMap(canonicalMap)
      writeFileSync(canonicalMapFile, JSON.stringify(canonicalMap))

      // The base sheet must wait for every file the bundler compiles, INCLUDING
      // the ones only a dynamic `import()` reaches. `computeFileRoutes` stops at
      // static edges (correct for hoisting, where a lazy chunk is its own unit),
      // so using its keys here resolved the wait before any `dynamic()` module
      // had extracted and shipped a sheet missing all of their atoms.
      expectedBaseFiles = computeCompiledFiles({
        srcDir,
        tsconfigPath,
        cwd,
        graph,
      })

      if (atomMode) {
        const fileRoutes = computeFileRoutes({
          srcDir,
          tsconfigPath,
          cwd,
          graph,
        })
        // Fold per-file route reach onto the canonical bucket so the keys match
        // the engine's property bucket keys (canonical(filename)).
        const plan = planAtomHoist(canonicalMap, fileRoutes, atomHoist)
        if (plan) {
          importFileRoutes(plan.reachByBucket)
          setAtomHoist(plan.threshold)
        } else {
          console.info(
            '[devup-ui] atomHoist is set but fewer than 2 routes were detected; atom hoisting is a no-op.',
          )
        }
      }
      reportProfile('next.graph', {
        durationMs: elapsedMs(graphStartedAt),
        files: staticGraph.files.length,
        expectedBaseFiles: expectedBaseFiles.length,
        wasmVariant,
      })
    } catch {
      // Pre-pass is best-effort; on failure canonical() is the identity (no
      // merge) and atom hoisting stays off.
      reportProfile('next.graph', {
        durationMs: elapsedMs(graphStartedAt),
        failed: true,
      })
    }

    // Turbopack can request a CSS module before it has scheduled every source
    // loader. Waiting for a quiet window is not a compilation-complete signal:
    // a CSS request can itself hold up the next extraction wave. In one-shot
    // builds, extract every source candidate plus accepted external package
    // entries synchronously first. The wider set covers template imports, MDX
    // dependencies and package-level globalCss (notably reset-css) that the
    // route graph cannot represent. Loader-time extraction uses the same
    // keys/options and is idempotent.
    const prewarmedFiles: string[] = []
    const prewarmedOutputs = new Map<string, PrewarmedOutput>()
    if (!watch && staticGraph) {
      const prewarmStartedAt = profileStart()
      let prewarmExtractMs = 0
      let prewarmReadMs = 0
      let prewarmSourceBytes = 0
      const cwd = process.cwd()
      // The same complete candidate set selected the WASM variant above. Reuse
      // it here instead of resolving source/package entries a second time,
      // while retaining any compiled-file fallback supplied by the graph pass.
      const prewarmFiles = [
        ...new Set([...wasmCandidateFiles, ...expectedBaseFiles]),
      ].sort()
      for (const filename of prewarmFiles) {
        const resourcePath = resolve(cwd, filename)
        const relCssDir = `./${relative(
          dirname(resourcePath),
          cssDir,
        ).replaceAll('\\', '/')}`
        const readStartedAt =
          prewarmStartedAt === undefined ? undefined : performance.now()
        const preparedVanilla = staticVanillaSources.get(filename)
        const source =
          preparedVanilla?.source ?? readFileSync(resourcePath, 'utf-8')
        if (readStartedAt !== undefined) {
          prewarmReadMs += performance.now() - readStartedAt
          prewarmSourceBytes += Buffer.byteLength(source)
        }
        const extractStartedAt =
          prewarmStartedAt === undefined ? undefined : performance.now()
        const output = takeExtractOutput(
          extract(
            filename,
            preparedVanilla?.code ?? source,
            libPackage,
            relCssDir,
            singleCss,
            false,
            true,
            importAliases as unknown as Record<string, string | null>,
          ),
        )
        if (extractStartedAt !== undefined) {
          prewarmExtractMs += performance.now() - extractStartedAt
        }
        prewarmedOutputs.set(filename, {
          code: output.code,
          cssFile: output.cssFile,
          map: output.map,
          source,
          updatedBaseStyle: output.updatedBaseStyle,
        })
        prewarmedFiles.push(filename)
      }
      reportProfile('next.prewarm', {
        collectMs: candidateCollectMs,
        durationMs: elapsedMs(prewarmStartedAt),
        extractMs:
          prewarmStartedAt === undefined
            ? undefined
            : Number(prewarmExtractMs.toFixed(2)),
        files: prewarmedFiles.length,
        readMs:
          prewarmStartedAt === undefined
            ? undefined
            : Number(prewarmReadMs.toFixed(2)),
        sourceBytes: prewarmSourceBytes,
      })
    }

    // create devup-ui.css file
    const initialCssStartedAt = profileStart()
    const initialCssSerializeStartedAt =
      initialCssStartedAt === undefined ? undefined : performance.now()
    const initialCss = getCss(null, false)
    const initialCssSerializeMs = elapsedMs(initialCssSerializeStartedAt)
    const initialCssWriteStartedAt =
      initialCssStartedAt === undefined ? undefined : performance.now()
    writeFileSync(join(cssDir, 'devup-ui.css'), initialCss)
    reportProfile('next.initialCss', {
      bytes:
        initialCssStartedAt === undefined
          ? undefined
          : Buffer.byteLength(initialCss),
      durationMs: elapsedMs(initialCssStartedAt),
      serializeMs: initialCssSerializeMs,
      writeMs: elapsedMs(initialCssWriteStartedAt),
    })

    // Delete stale port file from previous session so loaders don't connect
    // to a dead coordinator port. The new coordinator writes a fresh port file
    // once it starts listening.
    try {
      unlinkSync(coordinatorPortFile)
    } catch {
      // Port file doesn't exist (first run) — safe to ignore
    }

    const coordinator = startCoordinator({
      wasm,
      package: libPackage,
      cssDir,
      singleCss,
      sheetFile,
      classMapFile,
      fileMapFile,
      importAliases: importAliases as unknown as Record<string, string | null>,
      coordinatorPortFile,
      canonicalMap,
      expectedBaseFiles,
      prewarmedFiles,
      prewarmedOutputs,
      sourceMap,
      staticVanillaExtract,
    })

    // Cleanup on exit
    process.on('exit', () => {
      coordinator.close()
    })
    const stateSnapshotStartedAt = profileStart()
    const sheetSerializeStartedAt =
      stateSnapshotStartedAt === undefined ? undefined : performance.now()
    const defaultSheetJson = exportSheet()
    const sheetSerializeMs = elapsedMs(sheetSerializeStartedAt)
    const sheetParseStartedAt =
      stateSnapshotStartedAt === undefined ? undefined : performance.now()
    const defaultSheet = JSON.parse(defaultSheetJson)
    const sheetParseMs = elapsedMs(sheetParseStartedAt)

    const classMapSerializeStartedAt =
      stateSnapshotStartedAt === undefined ? undefined : performance.now()
    const defaultClassMapJson = exportClassMap()
    const classMapSerializeMs = elapsedMs(classMapSerializeStartedAt)
    const classMapParseStartedAt =
      stateSnapshotStartedAt === undefined ? undefined : performance.now()
    const defaultClassMap = JSON.parse(defaultClassMapJson)
    const classMapParseMs = elapsedMs(classMapParseStartedAt)

    const fileMapSerializeStartedAt =
      stateSnapshotStartedAt === undefined ? undefined : performance.now()
    const defaultFileMapJson = exportFileMap()
    const fileMapSerializeMs = elapsedMs(fileMapSerializeStartedAt)
    const fileMapParseStartedAt =
      stateSnapshotStartedAt === undefined ? undefined : performance.now()
    const defaultFileMap = JSON.parse(defaultFileMapJson)
    const fileMapParseMs = elapsedMs(fileMapParseStartedAt)
    reportProfile('next.stateSnapshot', {
      classMapBytes:
        stateSnapshotStartedAt === undefined
          ? undefined
          : Buffer.byteLength(defaultClassMapJson),
      classMapParseMs,
      classMapSerializeMs,
      durationMs: elapsedMs(stateSnapshotStartedAt),
      fileMapBytes:
        stateSnapshotStartedAt === undefined
          ? undefined
          : Buffer.byteLength(defaultFileMapJson),
      fileMapParseMs,
      fileMapSerializeMs,
      sheetBytes:
        stateSnapshotStartedAt === undefined
          ? undefined
          : Buffer.byteLength(defaultSheetJson),
      sheetParseMs,
      sheetSerializeMs,
    })
    // for theme script
    const defaultTheme = getDefaultTheme()
    if (defaultTheme) {
      process.env.DEVUP_UI_DEFAULT_THEME = defaultTheme
      config.env ??= {}
      Object.assign(config.env, {
        DEVUP_UI_DEFAULT_THEME: defaultTheme,
      })
    }

    const rules: NonNullable<typeof config.turbopack.rules> = {
      [`./${relative(process.cwd(), cssDir).replaceAll('\\', '/')}/*.css`]: [
        {
          loader: '@devup-ui/next-plugin/css-loader',
          options: {
            watch,
            coordinatorPortFile,
            sheetFile,
            classMapFile,
            fileMapFile,
            themeFile: devupFile,
            defaultSheet,
            defaultClassMap,
            defaultFileMap,
            theme,
          },
        },
      ],
      // Must cover every extension the import-graph pre-pass lists AND the
      // webpack rule matches (tsx|ts|jsx|js|mjs). Omitting one (e.g. jsx)
      // means those files are never extracted: their Box/styled markup hits
      // the runtime stubs ("Cannot run on the runtime") and the coordinator
      // waits on graph members that never POST /extract.
      '*.{tsx,ts,jsx,js,mjs}': {
        loaders: [
          {
            loader: '@devup-ui/next-plugin/loader',
            options: {
              package: libPackage,
              cssDir,
              coordinatorPortFile,
              sheetFile,
              classMapFile,
              fileMapFile,
              themeFile: devupFile,
              defaultSheet,
              defaultClassMap,
              defaultFileMap,
              watch,
              singleCss,
              // for turbopack, load theme is required on loader
              theme,
              importAliases: importAliases as unknown as Record<string, string>,
            },
          },
        ],
        condition: {
          not: {
            path: excludeRegex,
          },
        },
      },
    }
    Object.assign(config.turbopack.rules, rules)
    if (!watch) {
      storeProductionTurboSetup(setupHandoffFile, {
        defaultTheme,
        key: productionSetupKey,
        prewarmedFiles: prewarmedFiles.length,
        rules,
        wasmVariant,
      })
    }
    reportProfile('next.setup', {
      cacheHit: false,
      durationMs: elapsedMs(pluginStartedAt),
      pid: process.pid,
      prewarmedFiles: prewarmedFiles.length,
      singleCss,
      wasmVariant,
      watch,
    })
    return config
  }

  const { webpack } = config
  config.webpack = (config, _options) => {
    const { DevupUIWebpackPlugin } = loadWebpackPlugin()
    options.cssDir ??= resolve(
      _options.dev ? (options.distDir ?? 'df') : '.next/cache',
      `devup-ui_${_options.buildId}`,
    )
    config.plugins.push(
      new DevupUIWebpackPlugin({
        ...options,
        watch: _options.dev,
      }),
    )
    if (typeof webpack === 'function') return webpack(config, _options)
    return config
  }
  return config
}
