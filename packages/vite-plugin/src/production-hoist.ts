import { dirname, relative, resolve } from 'node:path'

import {
  buildCanonicalMap,
  computeFileReach,
  computeReachableFiles,
  isMdxSource,
  isSelectedSource,
  planAtomHoist,
  type PreparationInputs,
  type ProductionManifestContext,
  type ProductionManifestFile,
  type ReservationRow,
  type StaticImportGraph,
} from '@devup-ui/plugin-utils'
import type { ResolvedConfig } from 'vite'

import type { ProductionOptions } from './production-manifest'

type ImportObserver = NonNullable<ProductionManifestContext['onImports']>
type ImportKind = Parameters<ImportObserver>[1][number]['kind']
const importTopology = {
  static: { forward: 'staticImports', reverse: 'staticImporters' },
  dynamic: { forward: 'dynamicImports', reverse: null },
} as const satisfies Record<
  ImportKind,
  {
    readonly forward: 'staticImports' | 'dynamicImports'
    readonly reverse: 'staticImporters' | null
  }
>
export class ProductionImports {
  readonly imports = new Map<string, Parameters<ImportObserver>[1]>()
  readonly observe: ImportObserver = (path, targets) => {
    const combined = new Map(
      [...(this.imports.get(path) ?? []), ...targets].map((target) => [
        JSON.stringify([target.kind, target.path]),
        target,
      ]),
    )
    this.imports.set(path, [...combined.values()])
  }
}

export interface ProductionHoistContext {
  readonly files: readonly ProductionManifestFile[]
  readonly selected: readonly ProductionManifestFile[]
  readonly entries: readonly string[]
  readonly imports: ProductionImports['imports']
}

/** Main's physical reservation policy, projected into an owned production plan. */
export async function productionHoist(
  config: ResolvedConfig,
  options: ProductionOptions,
  preparation: {
    readonly contexts: readonly ProductionHoistContext[]
    readonly inputs: PreparationInputs
  },
) {
  const threshold = options.atomHoist ?? 2
  const files = [
    ...new Set(
      preparation.contexts.flatMap((context) =>
        [
          ...context.files.map((file) => resolve(file.path)),
          ...context.imports.keys(),
        ].map((file) => resolve(file)),
      ),
    ),
  ]
    .filter(
      (file) =>
        isSelectedSource(file, options.mdxExtensions) &&
        !/\.d\.[mc]?ts$/i.test(file),
    )
    .sort()
  const graph: StaticImportGraph = {
    files,
    fileSet: new Set(files),
    staticImports: new Map(files.map((file) => [file, new Set<string>()])),
    staticImporters: new Map(files.map((file) => [file, new Set<string>()])),
    dynamicImports: new Map(files.map((file) => [file, new Set<string>()])),
    dynamicTargets: new Set(),
  }
  const selected = new Map<string, ProductionManifestFile>()
  for (const context of preparation.contexts) {
    for (const file of context.selected) selected.set(file.id, file)
    for (const [path, imports] of context.imports) {
      const file = resolve(path)
      for (const reference of imports) {
        const target = resolve(reference.path)
        if (!graph.fileSet.has(target)) continue
        const topology = importTopology[reference.kind]
        graph[topology.forward].get(file)?.add(target)
        if (topology.reverse === null) graph.dynamicTargets.add(target)
        else graph[topology.reverse].get(target)?.add(file)
      }
    }
  }
  const cssDir = resolve(
    config.root,
    options.cssDir ?? `${options.distDir}/devup-ui`,
  )
  const slots = [...selected.values()]
    .sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0))
    .filter((file) => !isMdxSource(file.path, options.mdxExtensions))
    .map((input): ReservationRow => {
      const directory = relative(dirname(input.id), cssDir).replaceAll(
        '\\',
        '/',
      )
      const css = directory.startsWith('./') ? directory : `./${directory}`
      return [input.id, preparation.inputs.read(input.path), css]
    })
  if (options.singleCss || !Number.isFinite(threshold) || threshold <= 0)
    return { canonical: {}, routes: {}, threshold: undefined, slots }
  const graphOptions = {
    graph,
    cwd: config.root,
    srcDir: config.root,
    keyBy: 'absolute' as const,
  }
  const canonical = buildCanonicalMap(graphOptions)
  const roots = [
    ...new Set(
      preparation.contexts.flatMap((context) =>
        context.entries.map((entry) => resolve(entry)),
      ),
    ),
  ]
  if (roots.length) {
    const reachable = computeReachableFiles({
      graph,
      srcDir: config.root,
      entries: roots,
    })
    roots.push(...reachable.filter((file) => graph.dynamicTargets.has(file)))
  }
  const reach = computeFileReach({ ...graphOptions, entries: roots })
  const plan = planAtomHoist(canonical, reach, threshold)
  return {
    canonical,
    routes: plan?.reachByBucket ?? {},
    threshold: plan?.threshold,
    slots,
  }
}
