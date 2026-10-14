import { expect, it } from 'bun:test'

import type {
  ManifestJob,
  ManifestKind,
} from '../production-manifest-certificates'
import { ManifestWorklist } from '../production-manifest-worklist'

const paths = ['s', 'a', 'b', 'c', 'leaf'] as const
const kinds = ['file', 'package'] as const
class SourceFailure extends Error {
  readonly name = 'SourceFailure'
  constructor(readonly path: string) {
    super(`Source failed: ${path}`)
  }
}
function job(path: string, kind: ManifestKind): ManifestJob {
  return { path, realPath: path, kind }
}
function fixture(
  leafFirst: boolean,
  faults: readonly SourceFailure[] = [],
  origins: readonly ManifestJob[] = kinds.map((kind) => job('s', kind)),
) {
  const graph = new Map<string, readonly string[]>([
    ['s', ['a', 'c']],
    ['a', ['b']],
    ['b', leafFirst ? ['leaf', 'c'] : ['c', 'leaf']],
    ['c', ['b']],
    ['leaf', ['leaf']],
  ])
  const records: string[] = []
  const expansions: ManifestJob[] = []
  const list = new ManifestWorklist({
    admit: () => true,
    record: (source) => records.push(source.path),
    expand: async (source, request) => {
      expansions.push(source)
      const fault = faults.find((cause) => cause.path === source.path)
      if (fault) throw fault
      for (const path of graph.get(source.path) ?? [])
        request(job(path, source.kind))
    },
  })
  for (const origin of origins) list.seed(origin)
  return { list, records, expansions }
}

it.each([true, false])(
  'bounds every admitted information cell when leaf completes first=%s',
  async (leafFirst) => {
    // Given input-derived C=2, K=2, L=5 and I=1 (completed) or I=2 (pending).
    const contexts = Array.from({ length: 2 }, () => fixture(leafFirst))
    const information = leafFirst ? 1 : 2
    const bound = contexts.length * kinds.length * paths.length * information
    const universe = new Set<string>(paths)
    // When independent context instances traverse genuine roots and frozen batches.
    for (const context of contexts) await context.list.run()
    // Then all-created history, including queued completion shortcuts, fits the product.
    const created = contexts.reduce(
      (total, context) => total + context.list.information().length,
      0,
    )
    console.info(
      JSON.stringify({
        event: 'rooted-information',
        leafFirst,
        created,
        bound,
      }),
    )
    expect(created).toBeLessThanOrEqual(bound)
    for (const context of contexts) {
      const cells = context.list.information()
      const recorded = new Map<string, Set<string>>()
      for (const cell of cells) {
        const signature = JSON.stringify([
          cell.files.toSorted(),
          cell.packages.toSorted(),
        ])
        const previous = recorded.get(cell.key) ?? new Set<string>()
        expect(previous.has(signature)).toBe(false)
        previous.add(signature)
        recorded.set(cell.key, previous)
        expect(previous.size).toBeLessThanOrEqual(information)
        expect(
          [...cell.files, ...cell.packages].every((path) => universe.has(path)),
        ).toBe(true)
      }
      expect(recorded.size).toBe(kinds.length * paths.length)
      expect(new Set(context.records)).toEqual(universe)
      for (const kind of kinds)
        expect(
          context.expansions.filter(
            (source) => source.kind === kind && source.path === 'b',
          ),
        ).toHaveLength(information)
      if (!leafFirst)
        expect(cells.length).toBeGreaterThan(context.expansions.length)
    }
  },
)

it('propagates the original stable leaf fault when rooted descendants execute', async () => {
  // Given a genuine reachable source failure, not a repeat-only callback effect.
  const cause = new SourceFailure('leaf')
  const { list } = fixture(true, [cause])
  // When the required first leaf expansion runs.
  const completion = list.run()
  // Then no conditional evidence can suppress its original cause.
  await expect(completion).rejects.toBe(cause)
})

it.each([false, true])(
  'reports deterministic genuine-origin faults when registration reverses=%s',
  async (reverse) => {
    // Given two independently genuine faulty roots and identical input instances.
    const faults = [new SourceFailure('a'), new SourceFailure('leaf')]
    const ordered = reverse ? faults.toReversed() : faults
    const origins = ordered.map((cause) => job(cause.path, 'file'))
    const contexts = Array.from({ length: 2 }, () =>
      fixture(true, faults, origins),
    )
    const outcomes: unknown[] = []
    // When each instance executes the same ordered genuine-origin registration.
    for (const context of contexts)
      outcomes.push(await context.list.run().catch((cause: unknown) => cause))
    // Then the original first registered fault is deterministic across instances.
    for (const outcome of outcomes) expect(outcome).toBe(ordered[0])
  },
)

it.each([false, true])(
  'reports deterministic child faults when leaf batch precedence=%s',
  async (leafFirst) => {
    // Given two stable descendant faults in independently declared child-batch order.
    const leaf = new SourceFailure('leaf')
    const cycle = new SourceFailure('c')
    const contexts = Array.from({ length: 2 }, () =>
      fixture(leafFirst, [leaf, cycle]),
    )
    const outcomes: unknown[] = []
    // When identical frozen batches are traversed sequentially.
    for (const context of contexts)
      outcomes.push(await context.list.run().catch((cause: unknown) => cause))
    // Then first-fault selection follows the input order, not callback scheduling.
    for (const outcome of outcomes)
      expect(outcome).toBe(leafFirst ? leaf : cycle)
  },
)
