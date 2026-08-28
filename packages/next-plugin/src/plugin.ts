import {
  existsSync,
  mkdirSync,
  readFileSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs'
import { dirname, join, relative, resolve } from 'node:path'

import {
  buildCanonicalMap,
  buildStaticImportGraph,
  computeCompiledFiles,
  computeFileRoutes,
  createNodeModulesExcludeRegex,
  createThemeInterfaceArgs,
  loadDevupConfigSync,
  mergeImportAliases,
  planAtomHoist,
  type StaticImportGraph,
} from '@devup-ui/plugin-utils'
import {
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
} from '@devup-ui/wasm'
import {
  DevupUIWebpackPlugin,
  type DevupUIWebpackPluginOptions,
} from '@devup-ui/webpack-plugin'
import { type NextConfig } from 'next'

import {
  type PrewarmedOutput,
  startCoordinator,
  takeExtractOutput,
} from './coordinator'
import { collectProductionPrewarmFiles } from './prewarm'
import { elapsedMs, profileStart, reportProfile } from './profile'

type DevupUiNextPluginOptions = Omit<
  Partial<DevupUIWebpackPluginOptions>,
  'watch'
>

/**
 * Devup UI Next Plugin
 * @param config
 * @param options
 * @constructor
 */
export function DevupUI(
  config: NextConfig,
  options: DevupUiNextPluginOptions = {},
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

    registerShorthands(shorthands ?? {})

    if (prefix) {
      setPrefix(prefix)
    }

    const importAliases = mergeImportAliases(userImportAliases)

    const sheetFile = join(distDir, 'sheet.json')
    const classMapFile = join(distDir, 'classMap.json')
    const fileMapFile = join(distDir, 'fileMap.json')
    const canonicalMapFile = join(distDir, 'canonicalMap.json')
    const gitignoreFile = join(distDir, '.gitignore')
    if (!existsSync(distDir))
      mkdirSync(distDir, {
        recursive: true,
      })
    if (!existsSync(cssDir))
      mkdirSync(cssDir, {
        recursive: true,
      })
    if (!existsSync(gitignoreFile)) writeFileSync(gitignoreFile, '*')
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
    const watch = process.env.NODE_ENV === 'development'
    const sourceMap = watch || config.productionBrowserSourceMaps === true
    const extract = sourceMap ? codeExtract : codeExtractWithoutSourceMap
    // Hoisted out of the try so the coordinator can receive it for per-bucket
    // completion. Stays `{}` if the best-effort pre-pass fails.
    let canonicalMap: Record<string, string> = {}
    // Every runtime file the bundler will compile (cwd-relative POSIX) — the
    // deterministic base-css completion signal handed to the coordinator. Stays
    // `[]` (idle fallback) when no routes are detected or the pre-pass fails.
    let expectedBaseFiles: string[] = []
    let staticGraph: StaticImportGraph | undefined
    const graphStartedAt = profileStart()
    try {
      const srcDir = resolve(process.cwd(), 'src')
      const tsconfigPath = resolve(process.cwd(), 'tsconfig.json')
      const cwd = process.cwd()
      // One scan+parse of the source tree, shared by all three consumers below.
      const graph = buildStaticImportGraph(srcDir, tsconfigPath)
      staticGraph = graph
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
      const collectStartedAt =
        prewarmStartedAt === undefined ? undefined : performance.now()
      const prewarmFiles = collectProductionPrewarmFiles({
        cwd,
        graph: staticGraph,
        expectedBaseFiles,
        libPackage,
        include,
      })
      const collectDurationMs = elapsedMs(collectStartedAt)
      for (const filename of prewarmFiles) {
        const resourcePath = resolve(cwd, filename)
        const relCssDir = `./${relative(
          dirname(resourcePath),
          cssDir,
        ).replaceAll('\\', '/')}`
        const readStartedAt =
          prewarmStartedAt === undefined ? undefined : performance.now()
        const source = readFileSync(resourcePath, 'utf-8')
        if (readStartedAt !== undefined) {
          prewarmReadMs += performance.now() - readStartedAt
          prewarmSourceBytes += Buffer.byteLength(source)
        }
        const extractStartedAt =
          prewarmStartedAt === undefined ? undefined : performance.now()
        const output = takeExtractOutput(
          extract(
            filename,
            source,
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
        if (singleCss) {
          prewarmedOutputs.set(filename, {
            code: output.code,
            cssFile: output.cssFile,
            map: output.map,
            source,
            updatedBaseStyle: output.updatedBaseStyle,
          })
        }
        prewarmedFiles.push(filename)
      }
      reportProfile('next.prewarm', {
        collectMs: collectDurationMs,
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
    reportProfile('next.setup', {
      durationMs: elapsedMs(pluginStartedAt),
      prewarmedFiles: prewarmedFiles.length,
      singleCss,
      watch,
    })
    return config
  }

  const { webpack } = config
  config.webpack = (config, _options) => {
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
