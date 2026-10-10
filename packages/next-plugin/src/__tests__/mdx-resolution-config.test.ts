import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { createMdxSourceManager } from '../mdx-source-generation'
import { createWasm } from '../wasm'
import { paletteFixture } from './mdx-resolution-fixture'
import { sourceFixture, styledMdx } from './mdx-source-fixture'

it.each(['absent', 'inherited'] as const)(
  'refreshes aliases when the actual %s config input changes',
  async (kind) => {
    // Given
    const f = paletteFixture()
    const config = (color: string) =>
      JSON.stringify({
        compilerOptions: {
          baseUrl: '.',
          paths: { palette: [`node_modules/palette/${color}.js`] },
        },
      })
    if (kind === 'inherited') {
      f.write('tsconfig.json', '{"extends":"./base.json"}')
      f.write('base.json', config('red'))
    }
    const first = await f.manager.prepare(f.signal)
    const target = kind === 'absent' ? 'tsconfig.json' : 'base.json'
    expect(first.watchInputs).toContain(join(f.root, target))
    f.write(target, config('blue'))
    // When
    const next = await f.manager.refresh({
      generation: first,
      signal: f.signal,
    })
    // Then
    expect(f.counts()).toBe(2)
    expect(f.css(next)).toContain('color:blue')
    expect(f.css(next)).not.toContain('color:red')
  },
)

it('refreshes when an earlier actual tsconfig paths candidate appears', async () => {
  // Given
  const f = paletteFixture()
  f.write(
    'tsconfig.json',
    JSON.stringify({
      compilerOptions: {
        baseUrl: '.',
        paths: {
          palette: ['outside/earlier.js', 'node_modules/palette/red.js'],
        },
      },
    }),
  )
  const first = await f.manager.prepare(f.signal)
  f.write('outside/earlier.js', 'export const color = "blue"')
  // When
  const next = await f.manager.refresh({ generation: first, signal: f.signal })
  // Then
  expect(first.watchInputs).toContain(join(f.root, 'outside/earlier.js'))
  expect(f.css(next)).toContain('color:blue')
  expect(f.counts()).toBe(2)
})

it('rejects ordinary-only stale resolution proof at production finalization', async () => {
  // Given
  const f = paletteFixture(
    {
      'app/page.tsx': `import { css } from '@devup-ui/react'; import { color } from 'palette'; export const style = css({color})`,
    },
    false,
  )
  const first = await f.manager.prepare(f.signal)
  f.select('blue')
  // When / Then
  expect(() => f.manager.validateForCssFinalization(first)).toThrow(f.manifest)
})

it('does not reuse a legacy cache lacking resolution proof', async () => {
  // Given
  const f = paletteFixture()
  const first = await f.manager.prepare(f.signal)
  const legacy: unknown = JSON.parse(f.manager.restartCache(first))
  if (!Array.isArray(legacy)) throw new TypeError('Expected serialized entries')
  for (const entry of legacy) {
    if (typeof entry !== 'object' || entry === null)
      throw new TypeError('Expected serialized entry')
    Reflect.deleteProperty(entry, 'resolutionProofVersion')
  }
  // When
  await createMdxSourceManager(f.binding, JSON.stringify(legacy)).prepare(
    f.signal,
  )
  // Then
  expect(f.counts()).toBe(2)
})

it('keeps committed proof immutable when its configurer is installed on later engines', async () => {
  // Given
  const f = paletteFixture()
  const first = await f.manager.prepare(f.signal)
  const proof = first.resolutionInputs
  const fingerprint = proof.find(
    (input) => input.path === f.manifest,
  )?.fingerprint
  f.select('blue')
  // When
  first.configureWasm(createWasm(f.root))
  first.configureWasm(createWasm(f.root))
  const next = await f.manager.refresh({ generation: first, signal: f.signal })
  // Then
  expect(first.resolutionInputs).toBe(proof)
  expect(
    first.resolutionInputs.find((input) => input.path === f.manifest)
      ?.fingerprint,
  ).toBe(fingerprint)
  expect(
    next.resolutionInputs.find((input) => input.path === f.manifest)
      ?.fingerprint,
  ).not.toBe(fingerprint)
  expect(f.css(next)).toContain('color:blue')
})

it('retains actual unresolved package probes without inventing resolved request inputs', async () => {
  // Given
  const f = sourceFixture({
    'app/page.mdx': `import { unused } from 'not-installed'\n\n${styledMdx}`,
  })
  // When
  const generation = await f.manager.prepare(f.signal)
  // Then
  const request = generation.plan.graph.requests?.find(
    (request) => request.specifier === 'not-installed',
  )
  expect(request?.outcome).toEqual({
    kind: 'external',
    request: 'not-installed',
  })
  expect(generation.watchInputs).toContain(
    join(f.root, 'node_modules/not-installed/package.json'),
  )
  expect(f.css(generation)).toContain('background:red')
})
