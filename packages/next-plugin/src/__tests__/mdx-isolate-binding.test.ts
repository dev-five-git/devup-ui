import { createRequire } from 'node:module'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { createMdxIsolateOwner } from '../mdx-isolate-owner'
import { composeMdxRules, isMdxRecord } from '../mdx-pipeline'
import { compileMdx } from '../mdx-prepare'
import { fixture } from './mdx-boundary-fixture.test'

it('accepts actual stock composed RegExp metadata without losing its path condition', async () => {
  // Given
  const f = fixture('@next/mdx/mdx-js-loader')
  const installed = createRequire(join(f.root, 'package.json'))
  const previous = process.env.TURBOPACK
  process.env.TURBOPACK = '1'
  let config: unknown
  try {
    config = installed('@next/mdx')()({})
  } finally {
    if (previous === undefined) delete process.env.TURBOPACK
    else process.env.TURBOPACK = previous
  }
  if (
    !isMdxRecord(config) ||
    !isMdxRecord(config.turbopack) ||
    !isMdxRecord(config.turbopack.rules)
  )
    throw new TypeError('missing stock rules')
  const pipeline = composeMdxRules(
    { bundler: 'turbo', rules: config.turbopack.rules, aliases: {} },
    { loader: 'extract' },
  ).pipelines[0]
  if (!pipeline) throw new TypeError('missing composed pipeline')
  const owner = createMdxIsolateOwner(f.root)
  const canonical = await compileMdx({ ...f, pipeline })
  try {
    // When
    await owner.prepare(
      { run: 'stock', filename: f.filename, pipeline, timeoutMs: 5000 },
      f.signal,
    )
    // Then
    expect(
      (await owner.bind({ run: 'stock', filename: f.filename, canonical }))
        .source,
    ).toBe(canonical.source)
  } finally {
    await owner.dispose()
  }
})

it('closes bindings synchronously on disposal even for an already prepared result', async () => {
  // Given
  const f = fixture('@next/mdx/mdx-js-loader')
  const pipeline = { ...f.pipeline, bundler: 'turbo' as const }
  const canonical = await compileMdx(f)
  const owner = createMdxIsolateOwner(f.root)
  await owner.prepare(
    { run: 'one', filename: f.filename, pipeline, timeoutMs: 5000 },
    f.signal,
  )
  // When
  const closing = owner.dispose()
  const binding = owner.bind({ run: 'one', filename: f.filename, canonical })
  // Then
  await expect(binding).rejects.toThrow('disposed')
  await closing
})

it('retires run DATA without reviving the old run or replacing the worker', async () => {
  // Given
  const f = fixture('@next/mdx/mdx-js-loader')
  const pipeline = { ...f.pipeline, bundler: 'turbo' as const }
  const canonical = await compileMdx(f)
  const owner = createMdxIsolateOwner(f.root)
  const demand = { run: 'old', filename: f.filename, pipeline, timeoutMs: 5000 }
  await owner.prepare(demand, f.signal)
  try {
    // When
    const retiring = owner.retire('old')
    const binding = owner.bind({ run: 'old', filename: f.filename, canonical })
    const refused = expect(binding).rejects.toThrow('retired')
    await retiring
    await owner.prepare(demand, f.signal)
    await owner.prepare({ ...demand, run: 'new' }, f.signal)
    // Then
    await refused
    await expect(
      owner.bind({ run: 'old', filename: f.filename, canonical }),
    ).rejects.toThrow('retired')
    expect(
      (await owner.bind({ run: 'new', filename: f.filename, canonical }))
        .source,
    ).toBe(canonical.source)
  } finally {
    await owner.dispose()
  }
})
