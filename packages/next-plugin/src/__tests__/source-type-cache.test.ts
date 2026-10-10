import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { immutableInput, preparedInput } from '../coordinator-generation'
import { parseExtractRequest } from '../coordinator-http'
import { createInputLedger } from '../coordinator-ledger'
import { sourceFixture } from './source-type-fixture'

it('refuses a cached transform when identical bytes are requested under another mode', () => {
  // Given
  const ledger = createInputLedger(2)
  const input = {
    filename: 'value.mdown',
    resourcePath: '/value.mdown',
    source: 'export const color="red"',
    sourceType: 'compiled-mdx' as const,
    dependencies: [],
    stamps: {},
    backing: '',
  }
  const output = { code: 'transformed', updatedBaseStyle: false }
  ledger.accept(input, output)
  // When
  const ordinary = ledger.lookup(input.filename, input.source)
  const compiled = ledger.lookup(input.filename, input.source, 'compiled-mdx')
  // Then
  expect(ordinary).toBeUndefined()
  expect(compiled).toBe(output)
})

it('retains the mode when a staged generation replaces the transform cache', () => {
  // Given
  const ledger = createInputLedger(2)
  const input = {
    filename: 'value.mdown',
    resourcePath: '/value.mdown',
    source: 'export const color="red"',
    sourceType: 'compiled-mdx' as const,
    dependencies: [],
    stamps: {},
    backing: '',
  }
  const output = { code: 'transformed', updatedBaseStyle: false }
  // When
  ledger.stage([immutableInput(input)], new Map([[input.filename, output]]))()
  // Then
  expect(ledger.lookup(input.filename, input.source)).toBeUndefined()
  expect(ledger.lookup(input.filename, input.source, 'compiled-mdx')).toBe(
    output,
  )
})

it('keeps the newest mode-keyed staged outputs when the cache reaches its bound', () => {
  // Given
  const ledger = createInputLedger(1)
  const input = {
    filename: 'first.mdown',
    resourcePath: '/first.mdown',
    source: 'export const x=1',
    sourceType: 'compiled-mdx' as const,
    dependencies: [],
    stamps: {},
    backing: '',
  }
  const next = { ...input, filename: 'next.mdown' }
  const output = { code: 'transformed', updatedBaseStyle: false }
  // When
  ledger.stage(
    [input, next],
    new Map([
      [input.filename, output],
      [next.filename, output],
    ]),
  )()
  // Then
  expect(
    ledger.lookup(input.filename, input.source, 'compiled-mdx'),
  ).toBeUndefined()
  expect(ledger.lookup(next.filename, next.source, 'compiled-mdx')).toBe(output)
})

it.each(['raw-mdx', null, false, {}, 1])(
  'rejects invalid request mode %j at the wire boundary',
  (sourceType) => {
    // Given
    const body = JSON.stringify({
      filename: 'page.mdown',
      resourcePath: '/page.mdown',
      code: '# raw',
      sourceType,
    })
    // When / Then
    expect(() => parseExtractRequest(body)).toThrow('/extract:1:1:')
  },
)

it('retains compiled mode while default requests stay absent', () => {
  // Given
  const request = {
    filename: 'page.mdown',
    resourcePath: '/page.mdown',
    code: 'export const value=1',
  }
  // When
  const compiled = parseExtractRequest(
    JSON.stringify({ ...request, sourceType: 'compiled-mdx' }),
  )
  const ordinary = parseExtractRequest(JSON.stringify(request))
  // Then
  expect(compiled.sourceType).toBe('compiled-mdx')
  expect(ordinary).toEqual(request)
})

it('rejects native bytes that match a prepared source but lack its compiler mode', async () => {
  // Given
  const f = sourceFixture({ 'app/page.mdx': '# Test' })
  const generation = await f.manager.prepare(f.signal)
  const input = generation.sources[0]?.input
  if (!input) throw new Error('Expected prepared input')
  // When / Then
  expect(() =>
    preparedInput(generation, {
      filename: input.filename,
      resourcePath: join(f.root, input.filename),
      code: input.source,
    }),
  ).toThrow('prepared generation')
})
