import { readFileSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { MdxCompileError } from '../mdx-prepare'
import { createMdxSourceManager } from '../mdx-source-generation'
import { captureMdxTurboContext } from '../mdx-turbo-context'
import { sourceFixture, styledMdx } from './mdx-source-fixture'

function managerFixture() {
  const f = sourceFixture({ 'app/page.mdx': styledMdx })
  const loader = f.pipeline.loaders[0]
  if (!loader) throw new TypeError('missing compiler')
  const constructions = f.write('constructions.txt', '')
  const executions = f.write('executions.txt', '')
  const plugin = f.write(
    'counter.cjs',
    `module.exports = function(options) { const fs = require('node:fs'); fs.appendFileSync(options.constructions, 'x'); return function() { fs.appendFileSync(options.executions, 'x') } }`,
  )
  const context = {
    owner: {},
    generation: {},
    ...captureMdxTurboContext(loader.loader, {
      rootContext: f.root,
      sourceMap: true,
      _module: { __reserved: 'TurbopackContext' },
    }),
  }
  const pipeline = {
    ...f.pipeline,
    bundler: 'turbo' as const,
    loaders: [
      {
        ...loader,
        options: {
          ...f.compilerOptions,
          remarkPlugins: [[plugin, { constructions, executions }]],
        },
      },
      ...f.pipeline.loaders.slice(1),
    ],
  }
  let raw: string | undefined
  const binding = {
    ...f.binding,
    async selectPipeline() {
      return {
        context: { ...context, owner: {}, generation: {} },
        identity: pipeline.loaders,
        pipeline: raw
          ? {
              ...pipeline,
              loaders: [pipeline.loaders[0] ?? loader, { loader: raw }],
            }
          : pipeline,
      }
    },
  }
  return {
    ...f,
    binding,
    manager: createMdxSourceManager(binding),
    constructions: () => readFileSync(constructions, 'utf8').length,
    executions: () => readFileSync(executions, 'utf8').length,
    raw(path?: string) {
      raw = path
    },
  }
}

it('recompiles marker bytes on refresh/restart while retaining the local private processor', async () => {
  // Given
  const f = managerFixture()
  const first = await f.manager.prepare(f.signal)
  const saved = f.manager.restartCache(first)
  // When
  const next = await f.manager.refresh({ generation: first, signal: f.signal })
  const restart = await createMdxSourceManager(f.binding, saved).prepare(
    f.signal,
  )
  // Then
  expect(next).not.toBe(first)
  expect(restart.sources).toEqual(first.sources)
  expect(next.cacheReader(join(f.root, 'app/page.mdx'))).toEqual(
    first.cacheReader(join(f.root, 'app/page.mdx')),
  )
  expect(f.executions()).toBe(3)
  expect(f.constructions()).toBe(1)
})

it.each(['compiler', 'cacheable'] as const)(
  'preserves exact predecessor and blocks extraction on caught marker refresh refusal: %s',
  async (form) => {
    // Given
    const f = managerFixture()
    const refused = f.write(
      'refused.cjs',
      form === 'compiler'
        ? 'module.exports = function(source) { try { void this._compiler } catch (error) { if (!(error instanceof Error)) throw error }; return source }'
        : 'module.exports = function(source) { this.cacheable(false); return source }',
    )
    const first = await f.manager.prepare(f.signal)
    const before = f.manager.restartCache(first)
    const extracted = f.extractionCalls()
    f.raw(refused)
    // When
    const error = await f.manager
      .refresh({ generation: first, signal: f.signal })
      .catch((cause: unknown) => cause)
    // Then
    expect(error).toBeInstanceOf(MdxCompileError)
    expect(f.extractionCalls()).toBe(extracted)
    expect(f.manager.restartCache(first)).toBe(before)
    f.raw()
    const recovery = await f.manager.refresh({
      generation: first,
      signal: f.signal,
    })
    expect(recovery.sources).toEqual(first.sources)
    expect(f.constructions()).toBe(1)
  },
)
