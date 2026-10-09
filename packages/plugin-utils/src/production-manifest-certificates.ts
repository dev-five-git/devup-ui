import type { ProductionSourceFile } from './production-source-files'

export type ManifestKind = 'file' | 'package'
export interface ManifestJob extends ProductionSourceFile {
  readonly kind: ManifestKind
}
export interface ManifestAncestry {
  readonly files: ReadonlySet<string>
  readonly packages: ReadonlySet<string>
}
interface Frontier {
  readonly files: Iterable<string>
  readonly packages: Iterable<string>
}
export type ManifestEvidence =
  | { readonly kind: 'cut'; readonly job: ManifestJob }
  | { readonly kind: 'node'; readonly node: ManifestObligation }
  | { readonly kind: 'closed'; readonly witness: ManifestCertificate }
export interface ManifestEdge {
  readonly delta: ManifestJob
  readonly evidence: ManifestEvidence
}
export type ManifestState =
  | { readonly phase: 'pending' | 'expanding' }
  | { readonly phase: 'expanded'; readonly edges: readonly ManifestEdge[] }
  | { readonly phase: 'completed'; readonly witness: ManifestCertificate }
export interface ManifestObligation {
  readonly job: ManifestJob
  readonly ancestry: ManifestAncestry
  // Only the lifecycle accumulator mutates; entry ancestry and edge references stay fixed.
  state: ManifestState
}

export function manifestNamespace(kind: ManifestKind) {
  switch (kind) {
    case 'file':
      return 'files'
    case 'package':
      return 'packages'
    default:
      return kind satisfies never
  }
}
export function manifestJobKey(job: ManifestJob): string {
  return JSON.stringify([job.kind, job.path])
}
export function manifestCovers(a: Frontier, b: ManifestAncestry): boolean {
  return (
    [...a.files].every((path) => b.files.has(path)) &&
    [...a.packages].every((path) => b.packages.has(path))
  )
}

class ClosedClosure {
  readonly #frontier: ManifestAncestry
  readonly requirements: {
    readonly files: readonly string[]
    readonly packages: readonly string[]
  }
  constructor(
    readonly key: string,
    frontier: ManifestAncestry,
  ) {
    this.#frontier = {
      files: new Set(frontier.files),
      packages: new Set(frontier.packages),
    }
    this.requirements = Object.freeze({
      files: Object.freeze([...frontier.files]),
      packages: Object.freeze([...frontier.packages]),
    })
    Object.freeze(this)
  }
  matches(ancestry: ManifestAncestry): boolean {
    return manifestCovers(this.requirements, ancestry)
  }
  covers(other: ClosedClosure): boolean {
    return this.matches(other.#frontier)
  }
}
export type ManifestCertificate = ClosedClosure

export class ManifestClosureError extends Error {
  readonly name = 'ManifestClosureError'
  constructor(
    readonly reason: 'member-open' | 'dependency-open' | 'foreign-witness',
  ) {
    super(`Production manifest closure: ${reason}`)
  }
}

export class ManifestCertificates {
  readonly #completed: Map<string, readonly ClosedClosure[]>
  readonly #issued: WeakSet<ClosedClosure>

  constructor() {
    this.#completed = new Map()
    this.#issued = new WeakSet()
  }

  lookup(job: ManifestJob, ancestry: ManifestAncestry) {
    return this.#completed
      .get(manifestJobKey(job))
      ?.find((witness) => witness.matches(ancestry))
  }

  #requirements(witness: ClosedClosure): Frontier {
    if (!this.#issued.has(witness))
      throw new ManifestClosureError('foreign-witness')
    return witness.requirements
  }

  #frontier(
    evidence: ManifestEvidence,
    internal: ReadonlyMap<ManifestObligation, ManifestAncestry>,
  ): Frontier {
    switch (evidence.kind) {
      case 'cut': {
        const frontier = {
          files: new Set<string>(),
          packages: new Set<string>(),
        }
        frontier[manifestNamespace(evidence.job.kind)].add(
          evidence.job.realPath,
        )
        return frontier
      }
      case 'closed':
        return this.#requirements(evidence.witness)
      case 'node': {
        const frontier = internal.get(evidence.node)
        if (frontier) return frontier
        const state = evidence.node.state
        switch (state.phase) {
          case 'completed':
            return this.#requirements(state.witness)
          case 'pending':
          case 'expanding':
          case 'expanded':
            throw new ManifestClosureError('dependency-open')
          default:
            return state satisfies never
        }
      }
      default:
        return evidence satisfies never
    }
  }

  seal(
    component: readonly ManifestObligation[],
  ): ReadonlyMap<ManifestObligation, ManifestCertificate> {
    const internal = new Map<
      ManifestObligation,
      {
        readonly files: Set<string>
        readonly packages: Set<string>
        readonly edges: readonly ManifestEdge[]
      }
    >()
    for (const node of component) {
      const state = node.state
      switch (state.phase) {
        case 'expanded':
          internal.set(node, {
            files: new Set(),
            packages: new Set(),
            edges: state.edges,
          })
          break
        case 'pending':
        case 'expanding':
        case 'completed':
          throw new ManifestClosureError('member-open')
        default:
          state satisfies never
      }
    }
    // Calculation sets are private until every outside proof and the least fixed point succeeds.
    let changed = true
    while (changed) {
      changed = false
      for (const target of internal.values()) {
        for (const edge of target.edges) {
          const child = this.#frontier(edge.evidence, internal)
          const supplied = manifestNamespace(edge.delta.kind)
          for (const namespace of ['files', 'packages'] as const) {
            for (const path of child[namespace]) {
              if (namespace === supplied && path === edge.delta.realPath)
                continue
              if (target[namespace].has(path)) continue
              target[namespace].add(path)
              changed = true
            }
          }
        }
      }
    }
    const staged = new Map<ManifestObligation, ClosedClosure>()
    for (const [node, frontier] of internal)
      staged.set(node, new ClosedClosure(manifestJobKey(node.job), frontier))
    // No callbacks or awaits separate these publications, and retained handles remain immutable.
    for (const [node, candidate] of staged) {
      const previous = this.#completed.get(candidate.key) ?? []
      const covering = previous.find((witness) => witness.covers(candidate))
      const witness = covering ?? candidate
      if (!covering)
        this.#completed.set(
          candidate.key,
          previous
            .filter((other) => !candidate.covers(other))
            .concat(candidate),
        )
      this.#issued.add(witness)
      staged.set(node, witness)
    }
    return staged
  }
}
