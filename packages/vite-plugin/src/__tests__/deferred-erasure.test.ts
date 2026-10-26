import './deferred-contract-cases'
import './deferred-forwarding-cases'

import { expect, it } from 'bun:test'

import {
  fixture,
  obligation,
  request,
  root,
  target,
} from './deferred-erasure-fixture'

it('does not cross-certify another same-name environment or fallback', async () => {
  // Given a positive certificate in the exact owning environment.
  const value = await fixture()
  expect(() =>
    value.deferred.verify(value.environment, value.graph()),
  ).not.toThrow()
  // When verification uses a distinct object, Then it refuses despite equal name.
  const other = { name: 'same-name' }
  value.deferred.extracted(other, root, JSON.stringify([obligation]))
  value.deferred.extracted(
    value.deferred.fallback,
    root,
    JSON.stringify([obligation]),
  )
  expect(() => value.deferred.verify(other, value.graph())).toThrow(
    /outer-order-unproven/,
  )
  expect(() =>
    value.deferred.verify(value.deferred.fallback, value.graph()),
  ).toThrow(/outer-order-unproven/)
})

it('requires fresh edge, root facts, target facts and stamp in a new epoch', async () => {
  const value = await fixture()
  // Given previously successful evidence, When the next build starts.
  value.begin()
  expect(() => value.deferred.verify(value.environment, value.graph())).toThrow(
    /outer-result-unproven/,
  )
  await value.edge()
  value.observeRoot()
  value.observeTarget()
  // Then renewed facts cannot borrow the previous stamp.
  expect(() => value.deferred.verify(value.environment, value.graph())).toThrow(
    /forwarding-unproven/,
  )
})

it('requires a current final target even when current ownership exists', async () => {
  const value = await fixture()
  value.begin()
  await value.edge()
  value.observeRoot()
  value.deferred.resolution.succeeded(value.environment, target)
  expect(() => value.deferred.verify(value.environment, value.graph())).toThrow(
    /forwarding-unproven/,
  )
})

it.each([
  ['wrong extraction', { extraction: '/other.tsx' }],
  ['forwarding cut', { cutImporter: '/barrel.ts' }],
  ['changed cut request', { cutRequest: './other.mdx' }],
  ['named export', { exports: ['named'] }],
  ['nested export', { exports: ['default', 'nested'] }],
  ['original attributes', { attributes: true }],
])('keeps located refusal for %s', async (_name, change) => {
  const value = await fixture()
  value.deferred.extracted(
    value.environment,
    root,
    JSON.stringify([{ ...obligation, ...change }]),
  )
  expect(() =>
    value.deferred.verify(
      value.environment,
      value.graph({ ...obligation, ...change }),
    ),
  ).toThrow(/\.tsx:1:8: deferred default cannot use/)
})

it('rejects malformed cut exports at the existing metadata boundary', async () => {
  const value = await fixture()
  expect(() =>
    value.deferred.extracted(
      value.environment,
      root,
      JSON.stringify([{ ...obligation, cutExport: 'named' }]),
    ),
  ).toThrow(TypeError)
})

it.each([
  'absent-request',
  'unsupported-request',
  'missing-imported-id',
] as const)('retains authority checks for %s', async (kind) => {
  const value = await fixture()
  value.observeRoot({
    importedIds: kind === 'missing-imported-id' ? [] : [target],
    observation: {
      defaultCallable: false,
      unsupportedRequests: new Set(
        kind === 'unsupported-request' ? [request] : [],
      ),
      importOnlyRequests: new Set(
        kind === 'absent-request' ? ['./other.mdx'] : [request],
      ),
    },
  })
  expect(() => value.deferred.verify(value.environment, value.graph())).toThrow(
    /page\.tsx:1:8: deferred default cannot use/,
  )
})
