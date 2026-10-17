import { readFileSync } from 'node:fs'
import { dirname, extname, resolve } from 'node:path'

import {
  buildCanonicalMap,
  buildStaticImportGraph,
  collectProductionFileManifest,
  computeFileReach,
  createNodeModulesExcludeRegex,
  createNonphysicalIdStore,
  enumerateProductionSourceFiles,
  extractedNeedles,
  isSelectedSource,
  loadDevupConfig,
  type NonphysicalIdStore,
  planAtomHoist,
  type ProductionManifestFile,
  type ProductionNumberingPlan,
  type ResolutionInputObserver,
  resolveProjectPaths,
  resolveSourceDirs,
  tailwindCssFiles,
  withTailwindCss,
} from '@devup-ui/plugin-utils'
import type { ResolvedConfig } from 'vite'

import type { DevupUIPluginOptions } from './plugin'
import { productionEntryInputs } from './production-entry-inputs'
import { productionRootClosure } from './production-root-closure'
import { productionRootSources } from './production-root-sources'

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

export function createExtractionGate(
  options: Pick<
    ProductionOptions,
    'package' | 'include' | 'mdxExtensions' | 'importAliases'
  >,
) {
  const exclude = createNodeModulesExcludeRegex(options.include)
  const needles = extractedNeedles(options.package, options.importAliases)
  return {
    selected: (filename: string) =>
      isSelectedSource(filename, options.mdxExtensions) &&
      !exclude.test(filename),
    relevant: (code: string) => needles.some((needle) => code.includes(needle)),
    admission: (code: string) =>
      '@vanilla-extract/css' in options.importAliases &&
      ['@vanilla-extract/recipes', '@vanilla-extract/sprinkles'].some(
        (packageName) => code.includes(packageName),
      ),
  }
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
  const devup = await loadDevupConfig(paths.devupFile)
  const theme = withTailwindCss(devup.theme ?? {}, devup, config.root)
  const tailwindFiles = tailwindCssFiles(devup, config.root)
  const roots = resolveSourceDirs(config.root, options.sourceDirs)
  const configured = Object.entries(config.environments)
  const contexts: ProductionContext[] = []
  const gate = createExtractionGate(options)
  for (const [key, environment] of configured) {
    const { physicalEntries, selectedEntries, nonphysical, entryClosure } =
      productionEntryInputs(config.root, environment, options.mdxExtensions)
    const conditions = ['import', ...environment.resolve.conditions].map(
      (condition) =>
        condition === 'development|production'
          ? config.isProduction
            ? 'production'
            : 'development'
          : condition,
    )
    const graphRoots = [...new Set([...roots, ...selectedEntries.map(dirname)])]
    const exclude = [
      paths.distDir,
      paths.cssDir,
      resolve(config.root, environment.build.outDir),
    ]
    const discovered = await collectProductionFileManifest({
      contexts: [
        {
          key,
          files: enumerateProductionSourceFiles({
            roots: entryClosure ? [] : roots,
            ...(entryClosure
              ? { sourceFiles: selectedEntries }
              : { entries: physicalEntries }),
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
      entryClosure,
    })
    const rootFiles = entryClosure
      ? productionRootClosure(
          {
            key,
            files: productionRootSources(
              {
                roots,
                cwd: config.root,
                includeMdx: options.mdxExtensions,
                exclude,
              },
              options.include,
            ),
            resolverOptions: {
              cwd: config.root,
              conditions,
              includeMdx: options.mdxExtensions,
              onResolutionInputs: observe,
            },
            toId: (path) => path.replaceAll('\\', '/'),
          },
          { include: options.include, exclude },
          gate,
        )
      : []
    const memberships = new Map<string, ProductionManifestFile>()
    for (const file of [...discovered, ...rootFiles])
      memberships.set(JSON.stringify([file.context, file.path, file.id]), file)
    const compare = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0)
    const files = [...memberships.values()]
      .sort(
        (a, b) =>
          compare(a.realPath, b.realPath) ||
          compare(a.path, b.path) ||
          compare(a.context, b.context) ||
          compare(a.id, b.id),
      )
      .filter((file) => {
        if (!gate.selected(file.path) || /\.d\.[mc]?ts$/i.test(file.path))
          return false
        if (
          entryClosure &&
          options.mdxExtensions.includes(extname(file.path).toLowerCase())
        )
          return true
        const code = readFileSync(file.path, 'utf8')
        return gate.relevant(code)
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
  return {
    paths,
    root: config.root,
    theme,
    tailwindFiles,
    contexts: Object.freeze(contexts),
  }
}
