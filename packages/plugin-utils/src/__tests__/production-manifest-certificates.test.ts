import { expect, it } from 'bun:test'

import {
  type ManifestAncestry,
  type ManifestCertificate,
  ManifestCertificates,
  ManifestClosureError,
  type ManifestEdge,
  type ManifestJob,
  type ManifestKind,
  type ManifestObligation,
  type ManifestState,
} from '../production-manifest-certificates'

function ancestry(
  files: readonly string[] = [],
  packages: readonly string[] = [],
) {
  return { files: new Set(files), packages: new Set(packages) }
}
function job(kind: ManifestKind, path: string, realPath = path): ManifestJob {
  return { kind, path, realPath }
}
function node(
  descriptor: ManifestJob,
  incoming: ManifestAncestry = ancestry(),
): ManifestObligation {
  return { job: descriptor, ancestry: incoming, state: { phase: 'pending' } }
}
function cut(
  parent: ManifestObligation,
  kind: ManifestKind,
  path: string,
): ManifestEdge {
  return { delta: parent.job, evidence: { kind: 'cut', job: job(kind, path) } }
}
function link(
  parent: ManifestObligation,
  child: ManifestObligation,
): ManifestEdge {
  return { delta: parent.job, evidence: { kind: 'node', node: child } }
}
function closed(
  parent: ManifestObligation,
  witness: ManifestCertificate,
): ManifestEdge {
  return { delta: parent.job, evidence: { kind: 'closed', witness } }
}
function expanded(
  current: ManifestObligation,
  edges: readonly ManifestEdge[] = [],
) {
  current.state = { phase: 'expanded', edges }
}
function frontier(
  files: readonly string[] = [],
  packages: readonly string[] = [],
) {
  return { files, packages }
}
function complete(
  store: ManifestCertificates,
  nodes: readonly ManifestObligation[],
) {
  for (const [current, witness] of store.seal(nodes))
    current.state = { phase: 'completed', witness }
}

it('requires both external namespaces when irrelevant and extra ancestors differ', () => {
  // Given real cut evidence in both namespaces and unused original ancestry.
  const store = new ManifestCertificates()
  const current = node(
    job('file', 'entry'),
    ancestry(['x', 'unused'], ['x', 'unused']),
  )
  expanded(current, [cut(current, 'file', 'x'), cut(current, 'package', 'x')])
  // When its successfully expanded closure is sealed.
  complete(store, [current])
  // Then only both relevant frontiers authorize reuse, irrespective of unused/extra ancestors.
  expect(
    store.lookup(current.job, ancestry(['x', 'extra'], ['x', 'extra']))
      ?.requirements,
  ).toEqual(frontier(['x'], ['x']))
  expect(store.lookup(current.job, ancestry(['x'], []))).toBeUndefined()
  expect(store.lookup(current.job, ancestry([], ['x']))).toBeUndefined()
  expect(
    store.lookup(job('package', 'entry'), current.ancestry),
  ).toBeUndefined()
  expect(
    store.lookup(job('file', 'alias', 'entry'), current.ancestry),
  ).toBeUndefined()
  expect(
    new ManifestCertificates().lookup(current.job, current.ancestry),
  ).toBeUndefined()
})

it('keeps a sibling alias cut when the same physical source expanded elsewhere', () => {
  // Given a genuine left alias closure and a right cut under a distinct incoming ancestry.
  const store = new ManifestCertificates()
  const left = node(job('file', 'left/a', 'a'))
  expanded(left, [cut(left, 'file', 'a')])
  complete(store, [left])
  const parent = node(job('file', 'c'), ancestry(['a'], ['c']))
  expanded(parent, [
    cut(parent, 'file', 'a'),
    cut(parent, 'package', 'c'),
    link(parent, left),
  ])
  // When translated child evidence closes the parent.
  complete(store, [parent])
  // Then sibling expansion and a FILE c delta erase neither external a nor PACKAGE c.
  expect(store.lookup(parent.job, parent.ancestry)?.requirements).toEqual(
    frontier(['a'], ['c']),
  )
  expect(store.lookup(parent.job, ancestry([], ['c']))).toBeUndefined()
})

it('solves a mutual component when every local expansion and outside child is complete', () => {
  // Given an outside package witness and genuine mutually dependent file obligations.
  const store = new ManifestCertificates()
  const outside = node(job('package', 'z'), ancestry([], ['p']))
  expanded(outside, [cut(outside, 'package', 'p')])
  complete(store, [outside])
  const witness = store.lookup(outside.job, outside.ancestry)
  if (!witness) throw new TypeError('Expected outside completion')
  const a = node(job('file', 'a'), ancestry(['ext'], ['p']))
  const b = node(job('file', 'b'), ancestry(['ext', 'a'], ['p']))
  expanded(a, [link(a, b)])
  expanded(b, [
    link(b, a),
    closed(b, witness),
    cut(b, 'file', 'a'),
    cut(b, 'file', 'ext'),
  ])
  // When the whole component reaches its least fixed point before publication.
  complete(store, [a, b])
  // Then actual a transition removes only a locally; transitive external cuts survive.
  expect(store.lookup(a.job, a.ancestry)?.requirements).toEqual(
    frontier(['ext'], ['p']),
  )
  expect(store.lookup(b.job, b.ancestry)?.requirements).toEqual(
    frontier(['a', 'ext'], ['p']),
  )
  expect(store.lookup(a.job, ancestry([], ['p']))).toBeUndefined()
})

const openStates: readonly ManifestState[] = [
  { phase: 'pending' },
  { phase: 'expanding' },
  { phase: 'expanded', edges: [] },
]
it.each([...openStates])(
  'rejects an outside dependency when its phase is $phase',
  (state) => {
    // Given an expanded parent with a genuine but unclosed outside dependency.
    const store = new ManifestCertificates()
    const parent = node(job('file', 'parent'))
    const child = node(job('file', 'child'))
    child.state = state
    expanded(parent, [link(parent, child)])
    // When sealing attempts to use that incomplete child.
    expect(() => store.seal([parent])).toThrow(
      new ManifestClosureError('dependency-open'),
    )
    // Then no parent evidence was published.
    expect(store.lookup(parent.job, parent.ancestry)).toBeUndefined()
  },
)

it.each([{ phase: 'pending' }, { phase: 'expanding' }] as const)(
  'rejects component publication when a member is $phase',
  (state) => {
    // Given one locally expanded member and another unfinished member.
    const store = new ManifestCertificates()
    const ready = node(job('file', 'ready'))
    expanded(ready)
    const unfinished = node(job('file', 'unfinished'))
    unfinished.state = state
    // When the component is submitted before all local expansions succeed.
    expect(() => store.seal([ready, unfinished])).toThrow(
      new ManifestClosureError('member-open'),
    )
    // Then even the locally successful member has no certificate.
    expect(store.lookup(ready.job, ready.ancestry)).toBeUndefined()
  },
)

it('retains held immutable witnesses when completed antichain entries are replaced', () => {
  // Given two incomparable successful witnesses and a later identical covered witness.
  const store = new ManifestCertificates()
  const first = node(job('file', 'j'), ancestry(['x']))
  expanded(first, [cut(first, 'file', 'x')])
  complete(store, [first])
  const held = store.lookup(first.job, first.ancestry)
  if (!held) throw new TypeError('Expected held completion')
  const second = node(first.job, ancestry([], ['p']))
  expanded(second, [cut(second, 'package', 'p')])
  complete(store, [second])
  const same = node(first.job, first.ancestry)
  expanded(same, [cut(same, 'file', 'x')])
  complete(store, [same])
  expect(store.lookup(first.job, first.ancestry)).toBe(held)
  const empty = node(first.job)
  expanded(empty)
  const parent = node(job('file', 'parent'), ancestry(['x']))
  expanded(parent, [closed(parent, held)])
  // When a smaller completed frontier replaces lookup entries while a parent holds old evidence.
  complete(store, [empty, parent])
  // Then the old edge still supplies its frontier and all published requirements are immutable.
  expect(store.lookup(parent.job, parent.ancestry)?.requirements).toEqual(
    frontier(['x']),
  )
  expect(store.lookup(first.job, ancestry())?.requirements).toEqual(frontier())
  expect(
    Object.isFrozen(held.requirements) &&
      Object.isFrozen(held.requirements.files),
  ).toBe(true)
  expect(() => store.seal([parent])).toThrow(
    new ManifestClosureError('member-open'),
  )
})

it('rejects foreign closed evidence when stores represent different context instances', () => {
  // Given a real closed witness minted by a different context's store.
  const foreign = new ManifestCertificates()
  const child = node(job('file', 'child'))
  expanded(child)
  complete(foreign, [child])
  const witness = foreign.lookup(child.job, child.ancestry)
  if (!witness) throw new TypeError('Expected foreign completion')
  const store = new ManifestCertificates()
  const parent = node(job('file', 'parent'))
  expanded(parent, [closed(parent, witness)])
  // When the context attempts to consume that foreign witness.
  expect(() => store.seal([parent])).toThrow(
    new ManifestClosureError('foreign-witness'),
  )
  // Then no local completed evidence appears.
  expect(store.lookup(parent.job, parent.ancestry)).toBeUndefined()
})
