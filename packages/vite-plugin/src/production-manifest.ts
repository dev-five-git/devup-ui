import { existsSync } from 'node:fs'
import { extname, resolve } from 'node:path'

import {
  collectProductionFileManifest,
  createNodeModulesExcludeRegex,
  createNonphysicalIdStore,
  enumerateProductionSourceFiles,
  extractedNeedles,
  isSelectedSource,
  loadDevupConfig,
  type NonphysicalIdStore,
  PreparationInputs,
  type ProductionManifestFile,
  type ProductionNumberingPlan,
  type ReservationRow,
  type ResolutionInputObserver,
  resolveProjectPaths,
  resolveSourceDirs,
  tailwindCssFiles,
  withTailwindCss,
} from '@devup-ui/plugin-utils'
import type { ResolvedConfig } from 'vite'

import type { DevupUIPluginOptions } from './plugin'
import {
  productionEntryInputs,
  productionHtmlEntries,
} from './production-entry-inputs'
import {
  productionHoist,
  type ProductionHoistContext,
  ProductionImports,
} from './production-hoist'
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
  readonly slots: readonly ReservationRow[]
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
  const inputs = new PreparationInputs()
  try {
    return await prepareManifest(config, options, { observe, inputs })
  } finally {
    inputs.dispose()
  }
}

async function prepareManifest(
  config: ResolvedConfig,
  options: ProductionOptions,
  {
    observe,
    inputs,
  }: {
    readonly observe: ResolutionInputObserver
    readonly inputs: PreparationInputs
  },
) {
  const paths = resolveProjectPaths(config.root, {
    devupFile: options.devupFile,
    distDir: options.distDir,
    ...(options.cssDir === undefined ? {} : { cssDir: options.cssDir }),
  })
  const configFiles = new Set<string>()
  const devup = await loadDevupConfig(paths.devupFile, configFiles).finally(
    () => {
      observe({
        fileDependencies: [...configFiles].filter((path) => existsSync(path)),
        missingDependencies: [...configFiles].filter(
          (path) => !existsSync(path),
        ),
      })
    },
  )
  const theme = withTailwindCss(devup.theme ?? {}, devup, config.root)
  const tailwindFiles = tailwindCssFiles(devup, config.root)
  const roots = resolveSourceDirs(config.root, options.sourceDirs)
  const configured = Object.entries(config.environments)
  const exclude = [
    paths.distDir,
    paths.cssDir,
    ...configured.map(([, environment]) =>
      resolve(config.root, environment.build.outDir),
    ),
  ]
  const hoistContexts: ProductionHoistContext[] = []
  const contexts: Omit<
    ProductionContext,
    'canonical' | 'routes' | 'threshold' | 'slots'
  >[] = []
  const gate = createExtractionGate(options)
  for (const [key, environment] of configured) {
    const evidence = new ProductionImports()
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
    const entries = [...selectedEntries]
    if (!entryClosure)
      entries.push(
        ...(await productionHtmlEntries(config, environment, {
          inputs,
          conditions,
          mdxExtensions: options.mdxExtensions,
          observe,
        })),
      )
    const discovered = await collectProductionFileManifest(
      {
        contexts: [
          {
            key,
            files: enumerateProductionSourceFiles({
              roots: entryClosure ? [] : roots,
              ...(entryClosure
                ? { sourceFiles: selectedEntries }
                : { entries: [...physicalEntries, ...entries] }),
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
            onImports: evidence.observe,
          },
        ],
        include: options.include,
        exclude,
        entryClosure,
      },
      inputs,
    )
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
            onImports: evidence.observe,
          },
          { include: options.include, exclude },
          gate,
          inputs,
        )
      : []
    const memberships = new Map<string, ProductionManifestFile>()
    for (const file of [...discovered, ...rootFiles])
      memberships.set(JSON.stringify([file.context, file.path, file.id]), file)
    const compare = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0)
    const files = [...memberships.values()]
      .sort(
        (a, b) => compare(a.realPath, b.realPath) || compare(a.path, b.path),
      )
      .filter((file) => {
        if (!gate.selected(file.path) || /\.d\.[mc]?ts$/i.test(file.path))
          return false
        if (
          entryClosure &&
          options.mdxExtensions.includes(extname(file.path).toLowerCase())
        )
          return true
        const code = inputs.read(file.path)
        return gate.relevant(code)
      })
    hoistContexts.push({
      files: [...memberships.values()],
      selected: files,
      entries,
      imports: evidence.imports,
    })
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
        conditions,
        store,
        learned,
        scan: Object.freeze([...files.map((file) => file.id), ...nonphysical]),
      }),
    )
  }
  const hoist = await productionHoist(config, options, {
    contexts: hoistContexts,
    inputs,
  })
  return {
    paths,
    root: config.root,
    theme,
    tailwindFiles,
    configFiles: [...configFiles],
    contexts: Object.freeze(
      contexts.map((context): ProductionContext =>
        Object.freeze({ ...context, ...hoist }),
      ),
    ),
  }
}
