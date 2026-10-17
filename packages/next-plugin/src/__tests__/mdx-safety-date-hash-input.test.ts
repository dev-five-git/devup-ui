import { createRequire } from 'node:module'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { createMdxSourceManager } from '../mdx-source-generation'
import { fixture } from './mdx-boundary-fixture.test'
import { withNativeMdx } from './mdx-safety-native'
import { sourceFixture, styledMdx } from './mdx-source-fixture'

it('refreshes raw Date input outside MDX H with the same native processor and raw options identity', async () => {
  // Given
  const date = new Date('2026-01-01T00:00:00.000Z')
  const rawOptions = { date }
  let constructions = 0
  let processes = 0
  const attacher = () => {
    constructions++
    return () => {
      processes++
    }
  }
  const options = { format: 'mdx', remarkPlugins: [attacher] }
  const request = fixture('@mdx-js/loader', options)
  const f = sourceFixture({ 'app/page.mdx': styledMdx })
  const raw = f.write(
    'date-raw.cjs',
    'module.exports = function(source) { const options = this.getOptions(); module.exports.observe(options); return source + "\\n\\nraw-year-" + options.date.getUTCFullYear() }',
  )
  const rawModule: unknown = createRequire(join(f.root, 'package.json'))(raw)
  if (typeof rawModule !== 'function') throw new TypeError('missing raw loader')
  Reflect.set(rawModule, 'observe', (received: unknown) => {
    expect(received).toBe(rawOptions)
  })
  const pipeline = {
    ...request.pipeline,
    loaders: [
      ...request.pipeline.loaders,
      { loader: raw, options: rawOptions },
    ],
  }
  await withNativeMdx({ ...request, pipeline }, async (native) => {
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
    expect((await native.compile()).source).toContain('raw-year-2026')
    const first = await manager.prepare(f.signal)
    const filename = join(f.root, 'app/page.mdx')
    const before = first.cacheReader(filename)
    if (!before || typeof before === 'string')
      throw new TypeError('missing compiled source')
    expect(before.code).toContain('raw-year-2026')
    const count = constructions
    const hashInput = JSON.stringify(options)
    // When
    date.setUTCFullYear(2027)
    expect((await native.compile()).source).toContain('raw-year-2027')
    expect(constructions).toBe(count)
    expect(JSON.stringify(options)).toBe(hashInput)
    const processed = processes
    const next = await manager.refresh({ generation: first, signal: f.signal })
    // Then
    const after = next.cacheReader(filename)
    if (!after || typeof after === 'string')
      throw new TypeError('missing refreshed source')
    expect(after.code).toContain('raw-year-2027')
    expect(next).not.toBe(first)
    expect(before.code).toContain('raw-year-2026')
    expect(first.cacheReader(filename)).toEqual(before)
    expect(constructions).toBe(count)
    expect(processes).toBe(processed + 1)
  })
})

it.each(['nonenumerable', 'symbol', 'array-extra'] as const)(
  'refreshes a compiler Date under an unhashed %s field without reconstructing the processor',
  async (path) => {
    // Given
    const date = new Date('2026-01-01T00:00:00.000Z')
    const key = path === 'symbol' ? Symbol('date') : 'date'
    const pluginOptions = Object.defineProperty(
      path === 'array-extra' ? [] : {},
      key,
      { value: date, enumerable: path !== 'nonenumerable' },
    )
    let constructions = 0
    const attacher = (received: object) => {
      constructions++
      expect(received).toBe(pluginOptions)
      return (tree: {
        children: {
          type: string
          children: { type: string; value: string }[]
        }[]
      }) => {
        const live: unknown = Reflect.get(received, key)
        if (!(live instanceof Date)) throw new TypeError('missing live Date')
        expect(live).toBe(date)
        tree.children.push({
          type: 'paragraph',
          children: [
            { type: 'text', value: `hidden-year-${live.getUTCFullYear()}` },
          ],
        })
      }
    }
    const options = {
      format: 'mdx',
      remarkPlugins: [[attacher, pluginOptions]],
    }
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
      expect((await native.compile()).source).toContain('hidden-year-2026')
      const first = await manager.prepare(f.signal)
      const filename = join(f.root, 'app/page.mdx')
      const before = first.cacheReader(filename)
      if (!before || typeof before === 'string')
        throw new TypeError('missing compiled source')
      expect(before.code).toContain('hidden-year-2026')
      const count = constructions
      const hashInput = JSON.stringify(options)
      // When
      date.setUTCFullYear(2027)
      expect((await native.compile()).source).toContain('hidden-year-2027')
      expect(constructions).toBe(count)
      expect(JSON.stringify(options)).toBe(hashInput)
      const next = await manager.refresh({
        generation: first,
        signal: f.signal,
      })
      // Then
      const after = next.cacheReader(filename)
      if (!after || typeof after === 'string')
        throw new TypeError('missing refreshed source')
      expect(after.code).toContain('hidden-year-2027')
      expect(before.code).toContain('hidden-year-2026')
      expect(next).not.toBe(first)
      expect(constructions).toBe(count)
    })
  },
)
