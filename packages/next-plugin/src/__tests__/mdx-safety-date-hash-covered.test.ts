import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { rememberMdxCacheAdmission } from '../mdx-live-admission'
import { compileMdx } from '../mdx-prepare'
import {
  captureMdxCacheIdentity,
  compatibleMdxCache,
} from '../mdx-source-cache'
import { createMdxSourceManager } from '../mdx-source-generation'
import { fixture } from './mdx-boundary-fixture.test'
import { withNativeMdx } from './mdx-safety-native'
import { sourceFixture, styledMdx } from './mdx-source-fixture'

it('reuses live hash-covered compiler Date bytes but refreshes a changed timestamp', async () => {
  // Given
  const date = new Date('2026-01-01T00:00:00.000Z')
  let constructions = 0
  let processes = 0
  const attacher = (options: { readonly date: Date }) => {
    constructions++
    expect(options.date).toBe(date)
    return (tree: {
      children: { type: string; children: { type: string; value: string }[] }[]
    }) => {
      processes++
      tree.children.push({
        type: 'paragraph',
        children: [
          { type: 'text', value: `covered-${options.date.getUTCFullYear()}` },
        ],
      })
    }
  }
  const request = fixture('@mdx-js/loader', {
    format: 'mdx',
    remarkPlugins: [[attacher, { date }]],
  })
  const f = sourceFixture({ 'app/page.mdx': styledMdx })
  await withNativeMdx(request, async (native) => {
    const pipeline = { ...f.pipeline, loaders: request.pipeline.loaders }
    const manager = createMdxSourceManager({
      ...f.binding,
      async selectPipeline() {
        return {
          pipeline,
          identity: pipeline.loaders,
          context: { ...request.context, compiler: native.compiler },
        }
      },
    })
    expect((await native.compile()).source).toContain('covered-2026')
    const first = await manager.prepare(f.signal)
    const before = first.cacheReader(join(f.root, 'app/page.mdx'))
    const count = constructions
    const processed = processes
    expect(await manager.refresh({ generation: first, signal: f.signal })).toBe(
      first,
    )
    expect(processes).toBe(processed)
    expect(constructions).toBe(count)
    // When
    date.setUTCFullYear(2027)
    expect((await native.compile()).source).toContain('covered-2027')
    expect(constructions).toBe(count + 1)
    const next = await manager.refresh({ generation: first, signal: f.signal })
    // Then
    const after = next.cacheReader(join(f.root, 'app/page.mdx'))
    if (!after || typeof after === 'string')
      throw new TypeError('missing compiled source')
    expect(after.code).toContain('covered-2027')
    expect(next).not.toBe(first)
    expect(first.cacheReader(join(f.root, 'app/page.mdx'))).toEqual(before)
    expect(constructions).toBe(count + 2)
    expect(processes).toBe(processed + 2)
  })
})

it('does not treat a receipt over captured wire values as coverage of original Turbo Dates', async () => {
  // Given: an actual MDX receipt over serialized values, not a native Turbo claim.
  const date = new Date('2026-01-01T00:00:00.000Z')
  const request = fixture('@next/mdx/mdx-js-loader', {
    format: 'mdx',
    date: date.toJSON(),
  })
  const prepared = await compileMdx(request)
  const wire = {
    pipeline: request.pipeline,
    context: request.context,
    identity: request.pipeline.loaders,
  }
  const entry = { ...captureMdxCacheIdentity(wire), prepared, inputs: [] }
  rememberMdxCacheAdmission(entry, prepared, wire)
  expect(compatibleMdxCache(entry, wire)).toBe(true)
  const pipeline = {
    ...request.pipeline,
    bundler: 'turbo' as const,
    loaders: request.pipeline.loaders.map((loader) => ({
      ...loader,
      options: { format: 'mdx', date },
    })),
  }
  const selection = {
    pipeline,
    context: request.context,
    identity: pipeline.loaders,
  }
  const original = {
    ...captureMdxCacheIdentity(selection),
    prepared,
    inputs: [],
  }
  rememberMdxCacheAdmission(original, prepared, selection)
  // When
  const initial = compatibleMdxCache(original, selection)
  date.setUTCFullYear(2027)
  const current = compatibleMdxCache(original, selection)
  // Then
  expect(initial).toBe(false)
  expect(current).toBe(false)
  expect(prepared.source).toContain('original')
})
