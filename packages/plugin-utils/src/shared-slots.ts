import type {
  ReservationDiscoveryOptions,
  ReservationInput,
  ReservationInventory,
  ReservationResolutions,
} from './reservation-discovery'
import { collectReservationInputs } from './reservation-discovery'
import type { WasmImportAliases } from './types'

export type ReservationRow = readonly [
  filename: string,
  code: string,
  cssDir: string,
]

export interface ReservationEngine {
  reserveClassSlots(
    rows: readonly ReservationRow[],
    packageName: string,
    singleCss: boolean,
    importMainCssInCode: boolean,
    importMainCssInCss: boolean,
    importAliases: Readonly<WasmImportAliases>,
  ): void
}

export type ReservationBatchOptions = {
  readonly singleCss: boolean
  readonly importMainCssInCode: boolean
  readonly importMainCssInCss: boolean
  readonly cssDir: (input: ReservationInput) => string
}

export interface SharedSlotReservation {
  readonly dependencies: readonly string[]
  invalidate(): void
  reserve(
    discovery: ReservationDiscoveryOptions,
    options: ReservationBatchOptions,
  ): boolean
}

type Committed = {
  readonly inventory: ReservationInventory
  readonly rows: readonly ReservationRow[]
  readonly packageName: string
  readonly aliases: Readonly<WasmImportAliases>
  readonly singleCss: boolean
  readonly importMainCssInCode: boolean
  readonly importMainCssInCss: boolean
}

function sameSources(
  current: ReadonlyMap<string, string>,
  previous: ReadonlyMap<string, string>,
): boolean {
  return (
    current.size === previous.size &&
    [...current].every(([path, code]) => previous.get(path) === code)
  )
}

function sameRows(
  current: readonly ReservationRow[],
  previous: readonly ReservationRow[],
): boolean {
  return (
    current.length === previous.length &&
    current.every((row, index) =>
      row.every((value, field) => previous[index]?.[field] === value),
    )
  )
}

function sameResolutions(
  current: ReservationResolutions,
  previous: ReservationResolutions,
): boolean {
  return (
    current.entries.length === previous.entries.length &&
    current.entries.every(
      (target, index) => previous.entries[index] === target,
    ) &&
    current.imports.size === previous.imports.size &&
    [...current.imports].every(([importer, targets]) => {
      const prior = previous.imports.get(importer)
      return prior !== undefined && sameSources(targets, prior)
    })
  )
}

function sameAliases(
  current: Readonly<WasmImportAliases>,
  previous: Readonly<WasmImportAliases>,
): boolean {
  const keys = Object.keys(current)
  return (
    keys.length === Object.keys(previous).length &&
    keys.every((key) => current[key] === previous[key])
  )
}

/** Adapter-local revisions; an invalidation refreshes the entire closed inventory. */
export function createSharedSlotReservation(
  engine: ReservationEngine,
): SharedSlotReservation {
  let committed: Committed | undefined
  let dirty = true
  return {
    get dependencies() {
      return committed?.inventory.dependencies ?? []
    },
    invalidate() {
      dirty = true
    },
    reserve(discovery, options) {
      const inventory = collectReservationInputs(discovery)
      const rows: readonly ReservationRow[] = inventory.inputs.map(
        (input) => [input.filename, input.code, options.cssDir(input)] as const,
      )
      if (
        !dirty &&
        committed &&
        committed.packageName === discovery.package &&
        committed.singleCss === options.singleCss &&
        committed.importMainCssInCode === options.importMainCssInCode &&
        committed.importMainCssInCss === options.importMainCssInCss &&
        sameAliases(discovery.importAliases, committed.aliases) &&
        sameResolutions(
          inventory.resolutions,
          committed.inventory.resolutions,
        ) &&
        sameSources(inventory.sources, committed.inventory.sources) &&
        sameRows(rows, committed.rows)
      )
        return false
      engine.reserveClassSlots(
        rows,
        discovery.package,
        options.singleCss,
        options.importMainCssInCode,
        options.importMainCssInCss,
        discovery.importAliases,
      )
      committed = {
        inventory,
        rows,
        packageName: discovery.package,
        aliases: { ...discovery.importAliases },
        singleCss: options.singleCss,
        importMainCssInCode: options.importMainCssInCode,
        importMainCssInCss: options.importMainCssInCss,
      }
      dirty = false
      return true
    },
  }
}
