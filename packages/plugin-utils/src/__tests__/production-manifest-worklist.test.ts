import { expect, it } from 'bun:test'

import type {
  ManifestJob,
  ManifestKind,
} from '../production-manifest-certificates'
import { ManifestWorklist } from '../production-manifest-worklist'

function job(
  path: string,
  kind: ManifestKind = 'file',
  realPath = path,
): ManifestJob {
  return { kind, path, realPath }
}
class ExpansionFailure extends Error {
  readonly name = 'ExpansionFailure'
  constructor(readonly path: string) {
    super(`Expansion failed: ${path}`)
  }
}
function fixture(
  edges: readonly (readonly [ManifestJob, readonly ManifestJob[]])[],
  cause?: ExpansionFailure,
) {
  const graph = new Map<string, readonly ManifestJob[]>()
  for (const [source, targets] of edges)
    graph.set(JSON.stringify([source.kind, source.path]), targets)
  const records: string[] = []
  const expansions: ManifestJob[] = []
  const list = new ManifestWorklist({
    admit: (source) =>
      source.path !== 'excluded' && source.realPath !== 'blocked',
    record: (source) => records.push(source.path),
    expand: async (source, request) => {
      expansions.push(source)
      if (cause && source.path === cause.path) throw cause
      for (const target of graph.get(
        JSON.stringify([source.kind, source.path]),
      ) ?? [])
        request(target)
    },
  })
  return { list, records, expansions }
}

it('records boundary batches when file and package physical backedges cut expansion', async () => {
  // Given genuine initial origins, excluded descriptors and different lexical cut aliases.
  const a = job('a')
  const z = job('z')
  const p = job('p', 'package')
  const left = job('left')
  const right = job('right')
  const alias = job('alias', 'file', 'a')
  const { list, records, expansions } = fixture([
    [
      a,
      [alias, left, right, job('excluded'), job('hidden', 'file', 'blocked')],
    ],
    [p, [job('package-alias', 'package', 'p')]],
  ])
  for (const origin of [a, z, p, job('excluded')]) list.seed(origin)
  // When one iterative traversal executes the fully registered origins and child batches.
  await list.run()
  // Then membership precedes cuts and all local siblings precede descendant execution.
  expect(records).toEqual(['a', 'z', 'alias', 'left', 'right'])
  expect(expansions).toEqual([a, left, right, z, p])
})

it.each([false, true])(
  'retains both ancestry namespaces when branch order reverses=%s',
  async (reverse) => {
    // Given incomparable FILE x and PACKAGE x paths to the same lexical b.
    const s = job('s')
    const f = job('f', 'file', 'x')
    const p = job('p', 'package', 'x')
    const b = job('b')
    const fileAlias = job('file-alias', 'file', 'x')
    const packageAlias = job('package-alias', 'package', 'x')
    const { list, records, expansions } = fixture([
      [s, reverse ? [p, f] : [f, p]],
      [f, [b]],
      [p, [b]],
      [b, [fileAlias, packageAlias]],
      [fileAlias, [job('file-leaf')]],
      [packageAlias, [job('package-leaf')]],
    ])
    list.seed(s)
    // When neither namespace alone may dominate or certify the other path.
    await list.run()
    // Then both reopened leaves and exact lexical aliases survive in the genuine union.
    expect(new Set(records)).toEqual(
      new Set(['s', 'f', 'b', 'file-alias', 'file-leaf', 'package-leaf']),
    )
    expect(expansions.filter((source) => source.path === 'b')).toHaveLength(2)
  },
)

it.each([false, true])(
  'finishes mutual genuine cwd origins when an outside child fails=%s',
  async (fails) => {
    // Given empty-ancestry package origins covering requests made from a physical file alias.
    const s = job('s', 'file', 'f')
    const p = job('p', 'package')
    const q = job('q', 'package')
    const f = job('f')
    const g = job('g')
    const z = job('z')
    const cause = new ExpansionFailure('z')
    const { list, expansions } = fixture(
      [
        [s, [p]],
        [p, [f]],
        [f, [q]],
        [q, [g]],
        [g, [p, z]],
      ],
      fails ? cause : undefined,
    )
    for (const origin of [s, p, q]) list.seed(origin)
    // When pending covers execute their original genuine obligations instead of inherited ones.
    const completion = list.run()
    // Then all local cycle expansions are required and an outside failure remains fatal.
    if (fails) await expect(completion).rejects.toBe(cause)
    else await completion
    expect(expansions).toEqual([s, p, f, q, g, z])
  },
)

it('preserves active dependency references when a smaller genuine representative replaces them', async () => {
  // Given an inherited b and a genuine cwd chain capable of registering a smaller b.
  const s = job('s')
  const inherited = job('linked-p', 'package', 'p')
  const p = job('p', 'package')
  const q = job('q', 'package')
  const x = job('x')
  const b = job('b')
  const leaf = job('leaf')
  const { list, records, expansions } = fixture([
    [s, [inherited]],
    [inherited, [x]],
    [x, [b]],
    [b, [q, leaf]],
    [q, [p]],
    [p, [b]],
  ])
  for (const origin of [s, p, q]) list.seed(origin)
  // When q helps the original p while an older b dependency is still active.
  await list.run()
  // Then stable references finish both real b obligations and their finite leaf union.
  expect(expansions.filter((source) => source.path === 'b')).toHaveLength(2)
  expect(new Set(records)).toEqual(new Set(['s', 'x', 'b', 'leaf']))
})

it('forwards late completed evidence when a queued suffix later protects an external cut', async () => {
  // Given a queued j and a genuine h that completes an incomparable j first with frontier a.
  const s = job('s')
  const h = job('h')
  const t = job('t')
  const a = job('a')
  const alternate = job('alternate-a', 'file', 'a')
  const p = job('p')
  const j = job('j')
  const alias = job('alias-a', 'file', 'a')
  const leaf = job('leaf')
  const { list, records, expansions } = fixture([
    [s, [a]],
    [a, [p]],
    [p, [h, j]],
    [h, [alternate]],
    [alternate, [j]],
    [j, [alias]],
    [t, [p]],
    [alias, [leaf]],
  ])
  for (const origin of [s, h, t]) list.seed(origin)
  // When late queued completion must retain the closed witness on p's existing edge.
  await list.run()
  // Then later t reopens p and j rather than losing the alias-specific leaf.
  expect(new Set(records)).toEqual(
    new Set(['s', 'h', 't', 'a', 'alternate-a', 'p', 'j', 'alias-a', 'leaf']),
  )
  expect(expansions.filter((source) => source.path === 'p')).toHaveLength(2)
  expect(expansions.filter((source) => source.path === 'j')).toHaveLength(2)
})

it('separates kind and lexical keys when physical identities and origin records overlap', async () => {
  // Given two kinds sharing one string and two lexical file spellings sharing one physical ID.
  const file = job('same')
  const pkg = job('same', 'package')
  const alias = job('alias', 'file', 'same')
  const { list, records, expansions } = fixture([
    [file, [job('file-leaf')]],
    [pkg, [job('package-leaf')]],
    [alias, [job('alias-leaf')]],
  ])
  for (const origin of [file, pkg, alias, file]) list.seed(origin)
  // When repeated origins are registered genuinely before one collection run.
  await list.run()
  // Then no physical or kind winner discards a separately expanded source.
  expect(new Set(records)).toEqual(
    new Set(['same', 'alias', 'file-leaf', 'package-leaf', 'alias-leaf']),
  )
  expect(expansions).toHaveLength(6)
})

it('uses iterative traversal when a genuine two-origin chain has 4096 edges', async () => {
  // Given an independently declared flat owner graph and a generous expansion-work budget.
  const depth = 4096
  const chain = Array.from({ length: depth + 1 }, (_, index) =>
    job(String(index)),
  )
  const origins = [job('a'), job('b')]
  const declared = [...origins, ...chain]
  const graph: (readonly [ManifestJob, readonly ManifestJob[]])[] = []
  for (const origin of origins) graph.push([origin, chain.slice(0, 1)])
  for (const [index, source] of chain.entries())
    graph.push([source, chain.slice(index + 1, index + 2)])
  const { list, records, expansions } = fixture(graph)
  for (const origin of origins) list.seed(origin)
  // When the whole deep dependency graph is discharged without a recursive promise tree.
  await list.run()
  // Then every declared source appears and actual local expansion work stays graph-sized.
  expect(new Set(records)).toEqual(
    new Set(declared.map((source) => source.path)),
  )
  expect(new Set(expansions.map((source) => source.path))).toEqual(
    new Set(declared.map((source) => source.path)),
  )
  expect(expansions.length).toBeLessThanOrEqual(
    16 * (declared.length + depth + origins.length),
  )
  console.info(
    JSON.stringify({
      event: 'direct-depth',
      edges: depth,
      vertices: declared.length,
      expansions: expansions.length,
      requests: records.length,
    }),
  )
}, 120000)
