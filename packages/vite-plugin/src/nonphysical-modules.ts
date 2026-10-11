import { stat } from 'node:fs/promises'

import type {
  ModuleResolution,
  PreparedSource,
  ProductionManifestFile,
  ResolvedModule,
} from '@devup-ui/plugin-utils'
import { type Output, setModuleResolver } from '@devup-ui/wasm'

export interface NativePresence {
  getModuleIds(): IterableIterator<string>
  getModuleInfo(id: string): {
    readonly code: string | null
    readonly isEntry: boolean
    readonly importedIds: readonly string[]
    readonly dynamicallyImportedIds: readonly string[]
  } | null
}
type ReadySource = Exclude<PreparedSource, string | undefined>
export interface ExtractionRoot {
  readonly nativeId: string
  readonly id: string
  readonly source: ReadySource
}
type Attachments = ReadonlyMap<string, ReadonlyMap<string, ReadySource>>
interface Completion {
  readonly ids: readonly string[]
  readonly attachments: Attachments
}

function barePhysicalPath(id: string): string | undefined {
  return !/[\0?#]/.test(id) && /^(?:\/|[a-z]:\/)/i.test(id) ? id : undefined
}

async function exists(path: string): Promise<boolean> {
  try {
    return (await stat(path)).isFile()
  } catch (error) {
    if (
      error instanceof Error &&
      'code' in error &&
      (error.code === 'ENOENT' || error.code === 'ENOTDIR')
    )
      return false
    throw error
  }
}

export class NonphysicalModules {
  private readonly paths = new Map<string, string>()
  private readonly realPaths = new Map<string, string>()
  private readonly observed = new Set<string>()
  private readonly pending = new Map<string, Map<string, ReadySource>>()
  private completed: Attachments = new Map()

  constructor(files: readonly ProductionManifestFile[]) {
    for (const file of files) {
      this.paths.set(file.id, file.path)
      this.realPaths.set(file.id, file.realPath.replaceAll('\\', '/'))
    }
  }

  start(): void {
    this.observed.clear()
    this.pending.clear()
  }

  observations(): readonly string[] {
    return [...this.observed]
  }

  extract(
    root: ExtractionRoot,
    resolver: import('@devup-ui/plugin-utils').ModuleResolver,
    action: () => Output,
  ) {
    this.direct(root.nativeId, root.id, root.source)
    const observed = observeResolver(this, root.nativeId, resolver)
    setModuleResolver(observed)
    let output: Output
    try {
      output = action()
    } catch (error) {
      throw observed.remapError(error)
    }
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
      output.free()
    }
  }

  direct(nativeId: string, id: string, source: ReadySource): void {
    this.observed.add(id)
    this.pending.set(nativeId, new Map([[id, source]]))
    const path = barePhysicalPath(id)
    if (path !== undefined) this.paths.set(id, path)
  }

  resolved(nativeId: string, result: ModuleResolution | undefined): void {
    if (result === undefined) return
    switch (result.ignored) {
      case true:
        return
      case undefined: {
        this.observed.add(result.path)
        const path = barePhysicalPath(result.path)
        if (path !== undefined) this.paths.set(result.path, path)
        this.pending.get(nativeId)?.set(result.path, {
          code: result.code,
          ...(result.sourceType === undefined
            ? {}
            : { sourceType: result.sourceType }),
        })
        return
      }
      default:
        return result satisfies never
    }
  }

  source(path: string): PreparedSource {
    const id = path.replaceAll('\\', '/')
    for (const entries of this.pending.values()) {
      const ready = entries.get(id)
      if (ready !== undefined) return ready
    }
    for (const [root, entries] of this.completed) {
      if (this.pending.has(root)) continue
      const ready = entries.get(id)
      if (ready !== undefined) return ready
    }
    return undefined
  }

  invalidate(id: string): void {
    const matches = (root: string, entries: ReadonlyMap<string, ReadySource>) =>
      root === id ||
      [...entries.keys()].some(
        (key) =>
          key === id ||
          this.paths.get(key) === id ||
          this.realPaths.get(key) === id,
      )
    this.completed = new Map(
      [...this.completed].filter(([root, entries]) => !matches(root, entries)),
    )
    for (const [root, entries] of this.pending) {
      if (matches(root, entries)) this.pending.delete(root)
    }
  }

  async finish(
    graph: NativePresence,
    learned: readonly string[],
  ): Promise<Completion> {
    const reachable = new Set<string>()
    const present = new Set<string>()
    const queue = [...graph.getModuleIds()].filter(
      (id) => graph.getModuleInfo(id)?.isEntry,
    )
    for (let index = 0; index < queue.length; index += 1) {
      const id = queue[index]
      if (id === undefined || reachable.has(id)) continue
      reachable.add(id)
      const info = graph.getModuleInfo(id)
      if (info === null) continue
      if (typeof info.code === 'string') present.add(id)
      queue.push(...info.importedIds, ...info.dynamicallyImportedIds)
    }
    const attachments = new Map<string, ReadonlyMap<string, ReadySource>>()
    for (const root of [...present]) {
      const entries = this.pending.get(root) ?? this.completed.get(root)
      if (entries !== undefined) {
        attachments.set(root, new Map(entries))
        for (const id of entries.keys()) present.add(id)
      }
    }
    const ids = new Set(this.observed)
    for (const id of learned) {
      if (ids.has(id)) continue
      const path = this.paths.get(id) ?? barePhysicalPath(id)
      if (path === undefined ? present.has(id) : await exists(path)) ids.add(id)
    }
    return { ids: [...ids], attachments }
  }

  publish(completion: Completion): void {
    this.completed = completion.attachments
    this.pending.clear()
  }
}

export function observeResolver(
  modules: NonphysicalModules,
  nativeId: string,
  resolver: import('@devup-ui/plugin-utils').ModuleResolver,
): import('@devup-ui/plugin-utils').ModuleResolver {
  return Object.assign(
    (
      specifier: string,
      importer: string,
    ): ResolvedModule | { readonly ignored: true } | undefined => {
      const result = resolver(specifier, importer)
      modules.resolved(nativeId, result)
      return result
    },
    { remapError: resolver.remapError },
  )
}
