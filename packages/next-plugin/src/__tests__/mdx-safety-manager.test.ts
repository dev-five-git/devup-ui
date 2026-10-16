import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { MdxCompileError } from '../mdx-prepare'
import { createMdxSourceManager } from '../mdx-source-generation'
import { nativePreparationContext } from './mdx-safety-native'
import { sourceFixture, styledMdx } from './mdx-source-fixture'

it.each([false, true])(
  'reuses a selected ordinary MDX RegExp rule with a live function option=%s',
  async (liveFunction) => {
    // Given
    const f = sourceFixture({ 'app/page.mdx': styledMdx })
    if (liveFunction) f.pluginOptions.before = async () => undefined
    const context = nativePreparationContext(f.root)
    const pipeline = {
      ...f.pipeline,
      conditions: f.pipeline.conditions.map((rule) => ({
        ...rule,
        test: /\.mdx$/,
      })),
    }
    const manager = createMdxSourceManager({
      ...f.binding,
      async selectPipeline() {
        return {
          pipeline,
          identity: pipeline.loaders,
          context: { ...context, sourceMap: true, mode: 'production' },
        }
      },
    })
    const first = await manager.prepare(f.signal)
    const filename = join(f.root, 'app/page.mdx')
    const before = first.cacheReader(filename)
    expect(before).toMatchObject({ sourceType: 'compiled-mdx' })
    if (!before || typeof before === 'string')
      throw new TypeError('missing native compiled MDX')
    expect(before.code).toContain('<Box')
    expect(f.counts()).toBe(1)
    // When
    const next = await manager.refresh({ generation: first, signal: f.signal })
    // Then
    expect(f.counts()).toBe(1)
    expect(next).toBe(first)
    expect(next.cacheReader(filename)).toEqual(before)
  },
)

it('misses a changed original raw option under an unchanged selected MDX RegExp rule', async () => {
  // Given
  const f = sourceFixture({ 'app/page.mdx': styledMdx })
  const context = nativePreparationContext(f.root)
  let revision = 0
  const conditions = f.pipeline.conditions.map((rule) => ({
    ...rule,
    test: /\.mdx$/,
  }))
  const manager = createMdxSourceManager({
    ...f.binding,
    async selectPipeline() {
      const pipeline = {
        ...f.pipeline,
        conditions,
        loaders: f.pipeline.loaders.map((loader, index) =>
          index === 1
            ? {
                ...loader,
                options: {
                  ...(typeof loader.options === 'object' ? loader.options : {}),
                  revision,
                },
              }
            : loader,
        ),
      }
      return {
        pipeline,
        identity: pipeline.loaders,
        context: { ...context, sourceMap: true, mode: 'production' },
      }
    },
  })
  const first = await manager.prepare(f.signal)
  expect(f.counts()).toBe(1)
  // When
  revision = 1
  const next = await manager.refresh({ generation: first, signal: f.signal })
  // Then
  expect(f.counts()).toBe(2)
  expect(next).not.toBe(first)
  expect(next.cacheReader(join(f.root, 'app/page.mdx'))).toMatchObject({
    sourceType: 'compiled-mdx',
  })
})

it.each(['plugin', 'raw'])(
  'misses opaque RegExp %s input even with an ordinary selected MDX RegExp rule',
  async (input) => {
    // Given
    const f = sourceFixture({ 'app/page.mdx': styledMdx })
    const context = nativePreparationContext(f.root)
    const pattern = /mutable-input/g
    if (input === 'plugin') Reflect.set(f.pluginOptions, 'pattern', pattern)
    const pipeline = {
      ...f.pipeline,
      conditions: f.pipeline.conditions.map((rule) => ({
        ...rule,
        test: /\.mdx$/,
      })),
      loaders: f.pipeline.loaders.map((loader, index) =>
        input === 'raw' && index === 1
          ? {
              ...loader,
              options: {
                ...(typeof loader.options === 'object' ? loader.options : {}),
                pattern,
              },
            }
          : loader,
      ),
    }
    const manager = createMdxSourceManager({
      ...f.binding,
      async selectPipeline() {
        return {
          pipeline,
          identity: pipeline.loaders,
          context: { ...context, sourceMap: true, mode: 'production' },
        }
      },
    })
    const first = await manager.prepare(f.signal)
    expect(f.counts()).toBe(1)
    // When
    const next = await manager.refresh({ generation: first, signal: f.signal })
    // Then
    expect(f.counts()).toBe(2)
    expect(next).not.toBe(first)
    expect(next.cacheReader(join(f.root, 'app/page.mdx'))).toMatchObject({
      sourceType: 'compiled-mdx',
    })
  },
)

it('discards refused refresh candidates and recovers without modifying the exact predecessor', async () => {
  // Given
  const f = sourceFixture({ 'app/page.mdx': styledMdx }, {}, true)
  const raw = f.pipeline.loaders[1]
  if (!raw) throw new TypeError('missing raw loader')
  const body =
    'module.exports = function(source) { try { void this._compilation } catch (error) { if (!(error instanceof Error)) throw error }; return source }'
  const refusedRaw = f.write('refused-raw.cjs', body)
  const select = f.binding.selectPipeline
  let refuse = false
  const manager = createMdxSourceManager({
    ...f.binding,
    async selectPipeline(filename, signal) {
      const selected = await select(filename, signal)
      if (!selected || !refuse) return selected
      return {
        ...selected,
        pipeline: {
          ...selected.pipeline,
          loaders: [
            selected.pipeline.loaders[0] ?? raw,
            { loader: refusedRaw },
          ],
        },
      }
    },
  })
  const first = await manager.prepare(f.signal)
  const before = manager.restartCache(first)
  const extractionCalls = f.extractionCalls()
  refuse = true
  // When
  const refused = await manager
    .refresh({ generation: first, signal: f.signal })
    .catch((error: unknown) => error)
  // Then
  expect(refused).toBeInstanceOf(MdxCompileError)
  expect(f.extractionCalls()).toBe(extractionCalls)
  expect(manager.restartCache(first)).toBe(before)
  expect(first.cacheReader(join(f.root, 'app/page.mdx'))).toBeDefined()
  refuse = false
  const recovered = await manager.refresh({
    generation: first,
    signal: f.signal,
  })
  expect(recovered).toBe(first)
  expect(recovered.sources).toEqual(first.sources)
  const stable = await manager.refresh({
    generation: recovered,
    signal: f.signal,
  })
  expect(stable).toBe(recovered)
})

it('misses restart entries without process-private admission but reuses a stable live Compiler', async () => {
  // Given
  const f = sourceFixture({ 'app/page.mdx': styledMdx })
  const first = await f.manager.prepare(f.signal)
  const saved = f.manager.restartCache(first)
  const count = f.counts()
  // When
  const live = await f.manager.refresh({ generation: first, signal: f.signal })
  const restarted = await createMdxSourceManager(f.binding, saved).prepare(
    f.signal,
  )
  // Then
  expect(live).toBe(first)
  expect(f.counts()).toBe(count + 1)
  expect(restarted.sources).toEqual(first.sources)
})

it('misses a compiled source when its live native Compiler changes while preserving stable-scope refresh reuse', async () => {
  // Given
  const f = sourceFixture({ 'app/page.mdx': styledMdx })
  let context = nativePreparationContext(f.root)
  const manager = createMdxSourceManager({
    ...f.binding,
    async selectPipeline(filename, signal) {
      const selection = await f.binding.selectPipeline(filename, signal)
      return (
        selection && {
          ...selection,
          context: { ...selection.context, compiler: context.compiler },
        }
      )
    },
  })
  const first = await manager.prepare(f.signal)
  context = nativePreparationContext(f.root)
  // When
  const next = await manager.refresh({ generation: first, signal: f.signal })
  const stable = await manager.refresh({ generation: next, signal: f.signal })
  // Then
  expect(f.counts()).toBe(2)
  expect(next).not.toBe(first)
  expect(stable).toBe(next)
})
