import { resolve } from 'node:path'

import {
  computeReachableFiles,
  parseImports,
  type StaticImportGraph,
} from './import-graph'
import type { ReservationInventory } from './reservation-discovery'

/** Project only cached physical sources and resolved edges; never discover files. */
export function reservationGraph(
  inventory: ReservationInventory,
): StaticImportGraph {
  const sources = [...inventory.sources].sort(([a], [b]) =>
    a < b ? -1 : a > b ? 1 : 0,
  )
  const files = sources.map(([key]) => resolve(key))
  const fileSet = new Set(files)
  const staticImports = new Map<string, Set<string>>()
  const staticImporters = new Map<string, Set<string>>()
  const dynamicImports = new Map<string, Set<string>>()
  const dynamicTargets = new Set<string>()
  for (const file of files) {
    staticImports.set(file, new Set())
    staticImporters.set(file, new Set())
    dynamicImports.set(file, new Set())
  }
  for (const [key, code] of sources) {
    const file = resolve(key)
    const addEdge: Record<
      ReturnType<typeof parseImports>[number]['kind'],
      (target: string) => void
    > = {
      static(target) {
        staticImports.get(file)?.add(target)
        staticImporters.get(target)?.add(file)
      },
      dynamic(target) {
        dynamicImports.get(file)?.add(target)
        dynamicTargets.add(target)
      },
    }
    for (const reference of parseImports(file, code)) {
      const resolved = inventory.resolutions.imports
        .get(key)
        ?.get(reference.specifier)
      if (resolved === undefined) continue
      const target = resolve(resolved)
      if (!fileSet.has(target)) continue
      addEdge[reference.kind](target)
    }
  }
  return {
    files,
    fileSet,
    staticImports,
    staticImporters,
    dynamicImports,
    dynamicTargets,
  }
}

/**
 * The routes atom hoisting counts in a reservation graph, as `computeFileReach`
 * entries: the entries the bundler resolved and the files they lazy-load.
 * When the bundler names no entry, the project files no other file imports and
 * the lazy-loaded ones: the files of an included package are not routes.
 */
export function reservationRoutes(
  graph: StaticImportGraph,
  inventory: ReservationInventory,
): string[] {
  const entries = inventory.resolutions.entries.map((entry) => resolve(entry))
  if (entries.length === 0)
    return graph.files.filter(
      (file) =>
        !/[\\/]node_modules[\\/]/.test(file) &&
        (graph.staticImporters.get(file)?.size === 0 ||
          graph.dynamicTargets.has(file)),
    )
  return [
    ...entries,
    ...computeReachableFiles({ graph, srcDir: '', entries }).filter((file) =>
      graph.dynamicTargets.has(file),
    ),
  ]
}
