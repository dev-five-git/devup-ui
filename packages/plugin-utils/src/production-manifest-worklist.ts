import {
  type ManifestAncestry,
  ManifestCertificates,
  manifestCovers,
  type ManifestEdge,
  type ManifestEvidence,
  type ManifestJob,
  manifestJobKey,
  manifestNamespace,
  type ManifestObligation,
} from './production-manifest-certificates'
import type { ProductionSourceFile } from './production-source-files'

interface ManifestOwner {
  readonly admit: (job: ManifestJob) => boolean
  readonly record: (file: ProductionSourceFile) => void
  readonly expand: (
    job: ManifestJob,
    request: (child: ManifestJob) => void,
  ) => Promise<void>
}
// Tarjan visits and continuation frames are the engine's mutable accumulators.
interface Visit {
  readonly node: ManifestObligation
  readonly index: number
  lowlink: number
  active: boolean
}
interface Frame {
  readonly visit: Visit
  readonly edges: readonly ManifestEdge[]
  cursor: number
  child: Visit | undefined
}

export class ManifestWorklist {
  readonly #certificates = new ManifestCertificates()
  readonly #retained = new Map<string, readonly ManifestObligation[]>()
  readonly #origins: ManifestObligation[] = []
  readonly #visits = new Map<ManifestObligation, Visit>()
  readonly #components: Visit[] = []

  constructor(private readonly owner: ManifestOwner) {}

  #enqueue(
    job: ManifestJob,
    ancestry: ManifestAncestry,
  ): ManifestEvidence | undefined {
    if (!this.owner.admit(job)) return undefined
    switch (job.kind) {
      case 'file':
        this.owner.record({ path: job.path, realPath: job.realPath })
        break
      case 'package':
        break
      default:
        job.kind satisfies never
    }
    if (ancestry[manifestNamespace(job.kind)].has(job.realPath))
      return { kind: 'cut', job }
    const witness = this.#certificates.lookup(job, ancestry)
    if (witness) return { kind: 'closed', witness }
    const key = manifestJobKey(job)
    const previous = this.#retained.get(key) ?? []
    const covering = previous.find((node) =>
      manifestCovers(node.ancestry, ancestry),
    )
    if (covering) return { kind: 'node', node: covering }
    const node: ManifestObligation = {
      job: Object.freeze({ ...job }),
      ancestry,
      state: { phase: 'pending' },
    }
    this.#retained.set(
      key,
      previous
        .filter((other) => !manifestCovers(ancestry, other.ancestry))
        .concat(node),
    )
    return { kind: 'node', node }
  }

  seed(job: ManifestJob): void {
    const evidence = this.#enqueue(job, {
      files: new Set(),
      packages: new Set(),
    })
    if (!evidence) return
    switch (evidence.kind) {
      case 'node':
        this.#origins.push(evidence.node)
        break
      case 'cut':
      case 'closed':
        break
      default:
        evidence satisfies never
    }
  }

  async #enter(node: ManifestObligation): Promise<Frame | undefined> {
    if (this.#visits.has(node)) return undefined
    const witness = this.#certificates.lookup(node.job, node.ancestry)
    if (witness) {
      node.state = { phase: 'completed', witness }
      return undefined
    }
    const index = this.#visits.size
    const visit = { node, index, lowlink: index, active: true }
    this.#visits.set(node, visit)
    this.#components.push(visit)
    node.state = { phase: 'expanding' }
    const namespace = manifestNamespace(node.job.kind)
    const next = {
      ...node.ancestry,
      [namespace]: new Set(node.ancestry[namespace]).add(node.job.realPath),
    }
    const edges: ManifestEdge[] = []
    await this.owner.expand(node.job, (child) => {
      const evidence = this.#enqueue(child, next)
      if (evidence) edges.push({ delta: node.job, evidence })
    })
    node.state = { phase: 'expanded', edges: Object.freeze(edges) }
    return { visit, edges, cursor: 0, child: undefined }
  }

  async run(): Promise<void> {
    for (const origin of this.#origins) {
      const first = await this.#enter(origin)
      if (!first) continue
      const frames = [first]
      for (let frame = frames.at(-1); frame; frame = frames.at(-1)) {
        if (frame.child) {
          if (frame.child.active)
            frame.visit.lowlink = Math.min(
              frame.visit.lowlink,
              frame.child.lowlink,
            )
          frame.child = undefined
        }
        const edge = frame.edges[frame.cursor++]
        if (edge) {
          const evidence = edge.evidence
          switch (evidence.kind) {
            case 'node': {
              const previous = this.#visits.get(evidence.node)
              if (previous) {
                if (previous.active)
                  frame.visit.lowlink = Math.min(
                    frame.visit.lowlink,
                    previous.index,
                  )
              } else {
                const child = await this.#enter(evidence.node)
                if (child) {
                  frame.child = child.visit
                  frames.push(child)
                }
              }
              break
            }
            case 'cut':
            case 'closed':
              break
            default:
              evidence satisfies never
          }
          continue
        }
        if (frame.visit.lowlink === frame.visit.index) {
          const component: ManifestObligation[] = []
          for (
            let member = this.#components.pop();
            member;
            member = this.#components.pop()
          ) {
            member.active = false
            component.push(member.node)
            if (member === frame.visit) break
          }
          for (const [node, witness] of this.#certificates.seal(component))
            node.state = { phase: 'completed', witness }
        }
        frames.pop()
      }
    }
  }
}
