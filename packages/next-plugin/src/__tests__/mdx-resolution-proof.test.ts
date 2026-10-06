import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { takeExtractOutput } from '../coordinator-engine'
import { parseCoordinatorResponse } from '../loader-response'
import {
  readCompilerRestartReports,
  readResolutionRestartProof,
} from '../mdx-resolution-cache'
import {
  createResolutionProof,
  verifyResolutionProof,
} from '../mdx-resolution-proof'
import {
  createWasm,
  extractWithModuleResolver,
  withModuleResolver,
} from '../wasm'
import { paletteFixture } from './mdx-resolution-fixture'

it('captures callback failures without changing the producer error identity', () => {
  // Given
  const f = paletteFixture()
  const proof = createResolutionProof(f.root)
  // When
  expect(() =>
    proof.observe({
      fileDependencies: [join(f.root, 'unreadable.js')],
      missingDependencies: [],
    }),
  ).not.toThrow()
  // Then
  expect(() => proof.snapshot()).toThrow('reported file is not readable')
})

it('rejects a missing observation that is already present instead of stamping it fresh', () => {
  // Given
  const f = paletteFixture()
  const proof = createResolutionProof(f.root)
  // When
  proof.observe({ fileDependencies: [], missingDependencies: [f.manifest] })
  // Then
  expect(() => proof.snapshot()).toThrow('observed missing input appeared')
})

it('retires observation collectors without changing their captured proof', () => {
  // Given
  const f = paletteFixture()
  const proof = createResolutionProof(f.root)
  proof.observe({ fileDependencies: [f.manifest], missingDependencies: [] })
  const captured = proof.snapshot()
  proof.retire()
  // When
  proof.observe({
    fileDependencies: [join(f.root, 'unreadable.js')],
    missingDependencies: [],
  })
  f.select('blue')
  // Then
  expect(proof.snapshot()).toEqual(captured)
  expect(() => verifyResolutionProof(f.root, captured)).toThrow(f.manifest)
})

it('retires a failed actual extraction scope when a callback races the manifest', async () => {
  // Given
  const f = paletteFixture()
  const generation = await f.manager.prepare(f.signal)
  const engine = createWasm(f.root)
  generation.configureWasm(engine)
  let racing = true
  withModuleResolver(engine, f.root, {
    ...generation.resolver,
    onResolutionInputs(inputs) {
      if (racing && inputs.fileDependencies.includes(f.manifest))
        f.select('blue')
    },
  })
  const input = generation.sources[0]?.input
  if (!input) throw new TypeError('Missing prepared route')
  const extract = () =>
    extractWithModuleResolver(engine, false, [
      input.filename,
      input.source,
      '@devup-ui/react',
      './df',
      true,
      false,
      false,
      {},
    ])
  // When / Then
  expect(extract).toThrow(f.manifest)
  racing = false
  const output = takeExtractOutput(extract())
  expect(
    output.resolutionInputs?.some((input) => input.path === f.manifest),
  ).toBe(true)
  expect(engine.getCss(null, false)).toContain('color:blue')
})

it.each([null, {}, { fileDependencies: [1], missingDependencies: [] }])(
  'rejects malformed resolution transport %j',
  (resolutionInputs) => {
    // Given / When / Then
    expect(() =>
      parseCoordinatorResponse(
        JSON.stringify({ code: 'compiled', resolutionInputs }),
      ),
    ).toThrow(TypeError)
  },
)

it.each([
  null,
  [{}],
  [
    {
      kind: 'missing',
      path: '/candidate',
      loader: 'devup/resolution',
      fingerprint: 'present',
      mtime: 0,
    },
  ],
])('rejects an incomplete restart resolution proof %j', (value) => {
  // Given / When / Then
  expect(() => readResolutionRestartProof(value)).toThrow(TypeError)
})

it('keeps absent compiler reports distinct from invalid compiler reports', () => {
  // Given / When / Then
  expect(readCompilerRestartReports(undefined)).toEqual([])
  expect(() => readCompilerRestartReports(null)).toThrow(TypeError)
})

it('orders resolution proof by code points instead of host collation', () => {
  // Given
  const f = paletteFixture()
  const upper = f.write('outside/Z.ts', 'export {}')
  const lower = f.write('outside/a.ts', 'export {}')
  const proof = createResolutionProof(f.root)
  proof.observe({ fileDependencies: [lower, upper], missingDependencies: [] })
  // When
  const paths = proof.snapshot().map((input) => input.path)
  // Then
  expect(paths).toEqual([upper, lower])
})
