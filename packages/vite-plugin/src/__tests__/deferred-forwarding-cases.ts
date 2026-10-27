import { expect, it } from 'bun:test'

import { fixture, obligation, root, target } from './deferred-erasure-fixture'

it.each([
  'current',
  'require',
  'cycle',
  'absent-export',
  'missing-edge',
  'ambiguous-stars',
] as const)('verifies exact forwarding cuts when %s', async (kind) => {
  // Given current public resolver observations and final forwarding facts.
  const value = await fixture()
  value.begin()
  const forward = '/app/forward.js'
  const leaf = './leaf.mdx'
  const read = {
    ...obligation,
    request: './forward.js',
    exports: ['Doc'],
    cutImporter: forward,
    cutRequest: leaf,
  }
  value.deferred.extracted(value.environment, root, JSON.stringify([read]))
  await value.edge({ specifier: read.request, resolved: forward })
  value.observeRoot({
    importedIds: [forward],
    observation: { defaultCallable: false, unsupportedRequests: new Set() },
  })
  const ref = {
    request: leaf,
    ids: [kind === 'cycle' ? 'Doc' : 'default'],
    ...(kind === 'require' ? { kind: 'require-call' as const } : {}),
  }
  value.deferred.observe(value.environment, {
    id: forward,
    importedIds: [kind === 'cycle' ? forward : target],
    facts: {
      references: [],
      exports:
        kind === 'ambiguous-stars'
          ? new Map()
          : new Map([['Doc', kind === 'absent-export' ? undefined : ref]]),
      stars: kind === 'ambiguous-stars' ? ['./a.js', './b.js'] : [],
    },
    observation: { defaultCallable: false, unsupportedRequests: new Set() },
  })
  if (kind !== 'missing-edge')
    await value.edge({
      importer: forward,
      specifier: leaf,
      resolved: kind === 'cycle' ? forward : target,
    })
  value.observeTarget(true)
  value.deferred.resolution.succeeded(value.environment, target)
  // When the real verifier traverses those facts, Then only the unique supported cut succeeds.
  const verify = () =>
    value.deferred.verify(value.environment, value.graph(read))
  if (kind === 'current') expect(verify).not.toThrow()
  else expect(verify).toThrow(/page\.tsx:1:8:.*forwarding-unproven/)
})
