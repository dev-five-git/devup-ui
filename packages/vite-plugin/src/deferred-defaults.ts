import type { GuardFacts } from '@devup-ui/plugin-utils'

import {
  type DeferredDefault,
  DeferredDefaultError,
  deferredDefaults,
  DeferredMetadataError,
} from './deferred-metadata'
import { type FinalModuleGraph, FinalModules } from './final-modules'
import { OuterResolution } from './outer-resolution'

interface DeferredGraph extends FinalModuleGraph {
  getModuleIds(): IterableIterator<string>
  getModuleInfo(id: string):
    | (NonNullable<ReturnType<FinalModuleGraph['getModuleInfo']>> & {
        readonly meta?: Readonly<Record<string, unknown>>
      })
    | null
}

export class DeferredDefaults {
  readonly resolution = new OuterResolution()
  readonly fallback = {}
  private readonly contracts = new WeakMap<
    object,
    Map<
      string,
      {
        readonly serialized: string
        readonly records: readonly DeferredDefault[]
        readonly prefixes?: readonly string[]
      }
    >
  >()

  extracted(
    environment: object,
    id: string,
    serialized: string,
    diagnostic?: (prefix: string) => string,
  ): void {
    const records = deferredDefaults(serialized)
    let contracts = this.contracts.get(environment)
    if (!contracts) {
      contracts = new Map()
      this.contracts.set(environment, contracts)
    }
    contracts.set(id, {
      serialized,
      records,
      ...(diagnostic && records.length
        ? {
            prefixes: records.map((record) =>
              diagnostic(
                `${record.extraction}:${record.line}:${record.column}`,
              ),
            ),
          }
        : {}),
    })
  }

  verify(environment: object, graph: DeferredGraph): void {
    const modules = new FinalModules(graph)
    for (const id of graph.getModuleIds()) {
      const metadata = graph.getModuleInfo(id)?.meta?.['devupDeferredDefaults']
      const contract = this.contracts.get(environment)?.get(id)
      if (!contract) {
        if (metadata === undefined) continue
        if (typeof metadata === 'string') {
          let records: readonly DeferredDefault[]
          try {
            records = deferredDefaults(metadata)
          } catch (cause) {
            // JSON text creates plain data; this parser has no caller callbacks.
            throw new DeferredMetadataError(id, { cause })
          }
          const obligation = records[0]
          if (obligation)
            throw new DeferredDefaultError(obligation, 'metadata-unproven')
        }
        throw new DeferredMetadataError(id)
      }
      if (metadata !== contract.serialized) {
        const obligation = contract.records[0]
        if (obligation)
          throw new DeferredDefaultError(
            obligation,
            'metadata-unproven',
            contract.prefixes?.[0],
          )
        throw new DeferredMetadataError(id)
      }
      for (const [index, obligation] of contract.records.entries()) {
        const prefix = contract.prefixes?.[index]
        if (!this.resolution.valid(environment))
          throw new DeferredDefaultError(
            obligation,
            'outer-order-unproven',
            prefix,
          )
        if (obligation.attributes)
          throw new DeferredDefaultError(
            obligation,
            'outer-result-unproven',
            prefix,
          )
        const target = this.edge(
          environment,
          { importer: id, request: obligation.request },
          modules,
        )
        if (!target)
          throw new DeferredDefaultError(
            obligation,
            'outer-result-unproven',
            prefix,
          )
        const cut =
          obligation.extraction === obligation.cutImporter &&
          obligation.request === obligation.cutRequest
        if (id.split('?')[0] !== obligation.extraction)
          throw new DeferredDefaultError(
            obligation,
            'forwarding-unproven',
            prefix,
          )
        if (
          cut &&
          obligation.cutExport === 'default' &&
          obligation.exports.length === 1 &&
          obligation.exports[0] === 'default' &&
          modules
            .get(id)
            ?.observation?.importOnlyRequests?.has(obligation.request) &&
          modules.get(target) !== undefined &&
          this.resolution.owns(environment, target)
        )
          continue
        if (
          !this.discharge(
            environment,
            {
              id: target,
              exports: obligation.exports,
              cut,
              obligation,
              seen: new Set(),
            },
            modules,
          )
        )
          throw new DeferredDefaultError(
            obligation,
            'forwarding-unproven',
            prefix,
          )
      }
    }
  }

  private edge(
    environment: object,
    edge: { readonly importer: string; readonly request: string },
    modules: FinalModules,
  ): string | undefined {
    const module = modules.get(edge.importer)
    const target = this.resolution.edge(environment, edge)
    return target &&
      module?.importedIds.includes(target) &&
      module.observation !== undefined &&
      !module.observation.unsupportedRequests.has(edge.request)
      ? target
      : undefined
  }

  private discharge(
    environment: object,
    path: {
      readonly id: string
      readonly exports: readonly string[]
      readonly cut: boolean
      readonly obligation: DeferredDefault
      readonly seen: ReadonlySet<string>
    },
    modules: FinalModules,
  ): boolean {
    const key = JSON.stringify([path.id, path.exports])
    if (path.seen.has(key)) return false
    const module = modules.get(path.id)
    const name = path.exports[0]
    if (!module || !name) return false
    if (
      path.cut &&
      path.exports.length === 1 &&
      name === 'default' &&
      this.resolution.owns(environment, path.id) &&
      module.observation?.defaultCallable
    )
      return true
    const refs: readonly NonNullable<
      ReturnType<GuardFacts['exports']['get']>
    >[] = module.facts.exports.has(name)
      ? [module.facts.exports.get(name)].filter((value) => value !== undefined)
      : name === 'default'
        ? []
        : module.facts.stars.map((request) => ({ request, ids: [name] }))
    if (refs.length !== 1) return false
    const ref = refs[0]
    if (!ref || ref.kind === 'require-call') return false
    const target = this.edge(
      environment,
      { importer: path.id, request: ref.request },
      modules,
    )
    if (!target) return false
    const cut =
      path.cut ||
      (path.id === path.obligation.cutImporter &&
        ref.request === path.obligation.cutRequest &&
        ref.ids[0] === path.obligation.cutExport)
    return this.discharge(
      environment,
      {
        ...path,
        id: target,
        exports: [...ref.ids, ...path.exports.slice(1)],
        cut,
        seen: new Set([...path.seen, key]),
      },
      modules,
    )
  }
}
