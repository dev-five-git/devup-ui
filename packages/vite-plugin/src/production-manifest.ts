import { dirname, isAbsolute, resolve } from 'node:path'

import {
  buildCanonicalMap,
  buildStaticImportGraph,
  collectProductionFileManifest,
  computeFileReach,
  createNonphysicalIdStore,
  enumerateProductionSourceFiles,
  isSelectedSource,
  loadDevupConfig,
  type NonphysicalIdStore,
  planAtomHoist,
  type ProductionNumberingPlan,
  type ResolutionInputObserver,
  resolveProjectPaths,
  resolveSourceDirs,
} from '@devup-ui/plugin-utils'
import type { ResolvedConfig } from 'vite'

import type { DevupUIPluginOptions } from './plugin'

export type ProductionOptions = Readonly<
  Pick<
    DevupUIPluginOptions,
    'package' | 'debug' | 'include' | 'singleCss' | 'extractCss'
  >
> & {
  readonly devupFile: string
  readonly distDir: string
  readonly cssDir: string | undefined
  readonly sourceDirs: string | string[] | undefined
  readonly mdxExtensions: readonly string[]
  readonly prefix: string | undefined
  readonly shorthands: DevupUIPluginOptions['shorthands']
  readonly atomHoist: number | undefined
  readonly importAliases: import('@devup-ui/plugin-utils').WasmImportAliases
}
export interface ProductionContext extends ProductionNumberingPlan {
  readonly conditions: readonly string[]
  readonly store: NonphysicalIdStore
  readonly learned: readonly string[]
  readonly scan: readonly string[]
  readonly routes: Readonly<Record<string, readonly number[]>>
  readonly threshold: number | undefined
}

type Environment = ResolvedConfig['environments'][string]
function inputFor(root: string, environment: Environment): readonly string[] {
  const build = environment.build
  const native = build.rolldownOptions?.input ?? build.rollupOptions?.input
  const input = build.lib
    ? (native ?? build.lib.entry)
    : typeof build.ssr === 'string'
      ? build.ssr
      : (native ?? environment.input ?? resolve(root, 'index.html'))
  const entries =
    typeof input === 'string' ? [input] : Object.values(input ?? {})
  return entries.filter((entry): entry is string => typeof entry === 'string')
}

export async function prepareProductionManifest(
  config: ResolvedConfig,
  options: ProductionOptions,
  observe: ResolutionInputObserver,
) {
  const paths = resolveProjectPaths(config.root, {
    devupFile: options.devupFile,
    distDir: options.distDir,
    ...(options.cssDir === undefined ? {} : { cssDir: options.cssDir }),
  })
  const theme = (await loadDevupConfig(paths.devupFile)).theme ?? {}
  const roots = resolveSourceDirs(config.root, options.sourceDirs)
  const configured = Object.entries(config.environments)
  const contexts: ProductionContext[] = []
  for (const [key, environment] of configured) {
    const entries = inputFor(config.root, environment)
    const physicalEntries = entries.filter(
      (entry) =>
        !/[\0?#]/.test(entry) &&
        (isAbsolute(entry) || !/^[a-z][\w-]*:/i.test(entry)),
    )
    const nonphysical = entries
      .filter((entry) => !physicalEntries.includes(entry))
      .map((entry) => entry.replace(/\?.*$/s, ''))
    const conditions = ['import', ...environment.resolve.conditions].map(
      (condition) =>
        condition === 'development|production'
          ? config.isProduction
            ? 'production'
            : 'development'
          : condition,
    )
    const selectedEntries = physicalEntries
      .filter((entry) => isSelectedSource(entry, options.mdxExtensions))
      .map((entry) => resolve(config.root, entry))
    const graphRoots = [...new Set([...roots, ...selectedEntries.map(dirname)])]
    const exclude = [
      paths.distDir,
      paths.cssDir,
      resolve(config.root, environment.build.outDir),
    ]
    const files = await collectProductionFileManifest({
      contexts: [
        {
          key,
          files: enumerateProductionSourceFiles({
            roots,
            entries: physicalEntries,
            cwd: config.root,
            includeMdx: options.mdxExtensions,
            exclude,
          }),
          resolverOptions: {
            cwd: config.root,
            conditions,
            includeMdx: options.mdxExtensions,
            onResolutionInputs: observe,
          },
          toId: (path) => path.replaceAll('\\', '/'),
          unpreparedMarkdown: 'reserve-only',
        },
      ],
      include: options.include,
      exclude,
    })
    let canonical: Readonly<Record<string, string>> = {}
    let routes: Readonly<Record<string, readonly number[]>> = {}
    let threshold: number | undefined
    if (
      options.atomHoist !== undefined &&
      Number.isFinite(options.atomHoist) &&
      options.atomHoist > 0
    ) {
      const graph = buildStaticImportGraph(
        graphRoots,
        resolve(config.root, 'tsconfig.json'),
        {
          cwd: config.root,
          include: options.include,
          includeMdx: options.mdxExtensions,
          conditions,
          onResolutionInputs: observe,
        },
      )
      canonical = buildCanonicalMap({
        srcDir: graphRoots,
        cwd: config.root,
        keyBy: 'absolute',
        graph,
      })
      const reach = computeFileReach({
        srcDir: graphRoots,
        cwd: config.root,
        keyBy: 'absolute',
        graph,
        ...(selectedEntries.length ? { entries: selectedEntries } : {}),
      })
      const hoist = planAtomHoist(canonical, reach, options.atomHoist)
      if (hoist) {
        routes = hoist.reachByBucket
        threshold = hoist.threshold
      }
    }
    const store = createNonphysicalIdStore({
      integration: 'Vite',
      resolvedRoot: config.root,
      contextKey: key,
      distDir: paths.distDir,
    })
    const learned = (await store.read()).ids
    contexts.push(
      Object.freeze({
        context: key,
        files,
        nonphysical: Object.freeze([...nonphysical, ...learned]),
        canonical,
        conditions,
        routes,
        threshold,
        store,
        learned,
        scan: Object.freeze([...files.map((file) => file.id), ...nonphysical]),
      }),
    )
  }
  return { paths, theme, contexts: Object.freeze(contexts) }
}
