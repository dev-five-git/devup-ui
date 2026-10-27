import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import {
  createModulePathResolver,
  importedSpecifiers,
  isSourceFile,
  listSourceFiles,
} from './import-graph'
import { extractedNeedles, findPackageDir, isProjectJsx } from './numbering'
import {
  isReservationReadFailure,
  reservationSource,
} from './reservation-source'
import type { WasmImportAliases } from './types'

export type ReservationInput = {
  readonly physicalPath: string
  readonly filename: string
  readonly code: string
  readonly sourceType?: import('./prepared-source').SourceType
}

/** Internal compiled-input overlay for Devup-owned preparation, not a plugin option. */
export type PreparedReservationInput = ReservationInput

export type ReservationDiscoveryOptions = {
  readonly roots: readonly string[]
  readonly entries?: readonly {
    readonly request: string
    readonly context: string
  }[]
  readonly include?: readonly string[]
  readonly cwd?: string
  readonly tsconfigPath?: string
  readonly package: string
  readonly importAliases: Readonly<WasmImportAliases>
  readonly toId?: (physicalPath: string) => string
  readonly prepared?: readonly PreparedReservationInput[]
}

export type ReservationInventory = {
  readonly inputs: readonly ReservationInput[]
  readonly dependencies: readonly string[]
  readonly packageManifests: readonly string[]
  readonly sources: ReadonlyMap<string, string>
  readonly resolutions: ReservationResolutions
}

export type ReservationResolutions = {
  readonly entries: readonly string[]
  readonly imports: ReadonlyMap<string, ReadonlyMap<string, string>>
}

export class PreparedInputConflictError extends Error {
  readonly name = 'PreparedInputConflictError'
  constructor(readonly physicalPath: string) {
    super(`Conflicting prepared inputs for ${physicalPath}`)
  }
}

function physicalKey(path: string): string {
  return resolve(path).replaceAll('\\', '/')
}

function allowedPackage(
  specifier: string,
  include: readonly string[],
): boolean {
  return (
    specifier.startsWith('@devup-ui/') ||
    specifier.startsWith('@devup-editor/') ||
    include.some(
      (name) => specifier === name || specifier.startsWith(`${name}/`),
    )
  )
}

/** Isolate source failures in the prepass; real transforms retain their errors. */
export function collectReservationInputs(
  options: ReservationDiscoveryOptions,
): ReservationInventory {
  const cwd = resolve(options.cwd ?? process.cwd())
  const include = options.include ?? []
  const toId = options.toId ?? ((path: string) => path.replaceAll('\\', '/'))
  const overlays = new Map<string, PreparedReservationInput>()
  for (const prepared of options.prepared ?? []) {
    const key = physicalKey(resolve(cwd, prepared.physicalPath))
    const existing = overlays.get(key)
    if (
      existing &&
      (existing.filename !== prepared.filename ||
        existing.code !== prepared.code ||
        existing.sourceType !== prepared.sourceType)
    )
      throw new PreparedInputConflictError(key)
    overlays.set(key, { ...prepared, physicalPath: key })
  }
  const roots = options.roots.map((root) => resolve(cwd, root))
  for (const name of include) {
    const directory = reservationSource(() => findPackageDir(cwd, name))
    if (directory !== undefined) roots.push(directory)
  }
  const initial = roots.flatMap((root) =>
    listSourceFiles(root, [], {}, isReservationReadFailure),
  )
  const packageManifests = new Set<string>()
  const resolverOptions = {
    cwd,
    includeMdx: true,
    onPackageManifest: (path: string) =>
      packageManifests.add(physicalKey(path)),
    ...(options.tsconfigPath === undefined
      ? {}
      : { tsconfigPath: resolve(cwd, options.tsconfigPath) }),
  }
  const resolver = createModulePathResolver({
    ...resolverOptions,
    allowPackage: (specifier) => allowedPackage(specifier, include),
  })
  const needles = extractedNeedles(options.package, options.importAliases)
  const records = new Map<string, ReservationInput>()
  const sources = new Map<string, string>()
  const entryTargets = new Set<string>()
  const imports = new Map<string, Map<string, string>>()
  const edges = new Map<string, Set<string>>()
  const seeds = new Set(overlays.keys())
  const pending = [...initial, ...overlays.keys()]
  if (options.entries?.length) {
    const resolveEntry = createModulePathResolver(resolverOptions)
    for (const { request, context } of options.entries) {
      const module = reservationSource(
        () => resolveEntry(request, resolve(cwd, context, '__devup_entry__')),
        true,
      )
      if (!module || module.ignored === true) continue
      if (!isSourceFile(module.path) && !overlays.has(physicalKey(module.path)))
        continue
      entryTargets.add(physicalKey(module.path))
      pending.push(module.path)
    }
  }
  for (const physicalPath of pending) {
    const key = physicalKey(physicalPath)
    if (records.has(key)) continue
    const overlay = overlays.get(key)
    const code =
      overlay?.code ??
      sources.get(key) ??
      reservationSource(() => readFileSync(physicalPath, 'utf-8'))
    if (code === undefined) continue
    const filename = overlay?.filename ?? toId(physicalPath)
    sources.set(key, code)
    records.set(key, {
      physicalPath,
      filename,
      code,
      ...(overlay?.sourceType === undefined
        ? {}
        : { sourceType: overlay.sourceType }),
    })
    if (
      isProjectJsx(filename, options.importAliases) ||
      needles.some((needle) => code.includes(needle))
    )
      seeds.add(key)
    const targets = new Set<string>()
    const resolutions = new Map<string, string>()
    imports.set(key, resolutions)
    edges.set(key, targets)
    for (const specifier of importedSpecifiers(
      overlay?.filename ?? physicalPath,
      code,
      overlay?.sourceType,
    )) {
      const allowed = allowedPackage(specifier, include)
      const module = reservationSource(
        () => resolver(specifier, physicalPath),
        true,
      )
      if (!module || module.ignored === true) continue
      if (!isSourceFile(module.path) && !overlays.has(physicalKey(module.path)))
        continue
      const target = physicalKey(module.path)
      resolutions.set(specifier, target)
      targets.add(target)
      if (allowed) seeds.add(target)
      pending.push(module.path)
    }
  }
  const selected = new Set<string>()
  const closure = [...seeds]
  for (const key of closure) {
    if (selected.has(key)) continue
    selected.add(key)
    closure.push(...(edges.get(key) ?? []))
  }
  const dependencies = [...records.keys()].sort()
  const inputs = [...records]
    .filter(([key]) => selected.has(key))
    .sort(([left], [right]) => (left < right ? -1 : left > right ? 1 : 0))
    .map(([, record]) => record)
  return {
    inputs,
    dependencies,
    packageManifests: [...packageManifests].sort(),
    sources,
    resolutions: { entries: [...entryTargets].sort(), imports },
  }
}
