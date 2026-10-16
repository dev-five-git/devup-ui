import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { createMdxSourceManager } from '../mdx-source-generation'
import { fixture } from './mdx-boundary-fixture.test'
import { withNativeMdx } from './mdx-safety-native'
import { sourceFixture, styledMdx } from './mdx-source-fixture'

it.each(['change', 'remove'])(
  'refreshes compiled bytes when Date own data can %s without changing its native hash',
  async (mutation) => {
    // Given
    const date: Date & { text?: string } = Object.assign(
      new Date('2026-01-01T00:00:00.000Z'),
      { text: 'date-a' },
    )
    let constructions = 0
    const attacher = (options: {
      readonly date: Date & { readonly text?: string }
    }) => {
      constructions++
      expect(options.date).toBe(date)
      return (tree: {
        children: {
          type: string
          children: { type: string; value: string }[]
        }[]
      }) => {
        tree.children.push({
          type: 'paragraph',
          children: [{ type: 'text', value: options.date.text ?? 'date-b' }],
        })
      }
    }
    const options = { format: 'mdx', remarkPlugins: [[attacher, { date }]] }
    const request = fixture('@mdx-js/loader', options)
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
      const nativeBefore = await native.compile()
      const first = await manager.prepare(f.signal)
      const before = first.cacheReader(join(f.root, 'app/page.mdx'))
      if (!before || typeof before === 'string')
        throw new TypeError('missing compiled source receipt')
      expect(before.code).toContain('date-a')
      expect(nativeBefore.source).toContain('date-a')
      const count = constructions
      const nativeHashInput = JSON.stringify(options)
      // When
      if (mutation === 'remove') delete date.text
      else date.text = 'date-b'
      const nativeAfter = await native.compile()
      expect(nativeAfter.source).toContain('date-b')
      expect(JSON.stringify(options)).toBe(nativeHashInput)
      expect(constructions).toBe(count)
      const next = await manager.refresh({
        generation: first,
        signal: f.signal,
      })
      // Then
      const after = next.cacheReader(join(f.root, 'app/page.mdx'))
      if (!after || typeof after === 'string')
        throw new TypeError('missing refreshed source receipt')
      expect(after.code).toContain('date-b')
      expect(before.code).toContain('date-a')
      expect(next).not.toBe(first)
      expect(constructions).toBe(count)
    })
  },
)

it('refreshes compiled bytes when the same live Map changes without changing the native hash', async () => {
  // Given
  const map = new Map([['text', 'map-a']])
  let constructions = 0
  const attacher = (options: { readonly map: Map<string, string> }) => {
    constructions++
    expect(options.map).toBe(map)
    return (tree: {
      children: { type: string; children: { type: string; value: string }[] }[]
    }) => {
      tree.children.push({
        children: [{ type: 'text', value: options.map.get('text') ?? '' }],
        type: 'paragraph',
      })
    }
  }
  const options = { format: 'mdx', remarkPlugins: [[attacher, { map }]] }
  const request = fixture('@mdx-js/loader', options)
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
    const nativeBefore = await native.compile()
    const first = await manager.prepare(f.signal)
    const before = first.cacheReader(join(f.root, 'app/page.mdx'))
    if (!before || typeof before === 'string')
      throw new TypeError('missing compiled source receipt')
    expect(before?.code).toContain('map-a')
    expect(nativeBefore.source).toContain('map-a')
    const count = constructions
    const nativeHashInput = JSON.stringify(options)
    // When
    map.set('text', 'map-b')
    const nativeAfter = await native.compile()
    expect(nativeAfter.source).toContain('map-b')
    expect(JSON.stringify(options)).toBe(nativeHashInput)
    expect(constructions).toBe(count)
    const next = await manager.refresh({ generation: first, signal: f.signal })
    // Then
    const after = next.cacheReader(join(f.root, 'app/page.mdx'))
    if (!after || typeof after === 'string')
      throw new TypeError('missing refreshed source receipt')
    expect(after.code).toContain('map-b')
    expect(before?.code).toContain('map-a')
    expect(next).not.toBe(first)
    expect(constructions).toBe(count)
  })
})
