import { writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'

import { expect, it } from 'bun:test'

import { compileMdx } from '../mdx-prepare'
import {
  type MdxPreparationContext,
  mdxPrewarmCacheOwner,
} from '../mdx-prewarm-boundary'
import { fixture, type RemarkTree } from './mdx-boundary-fixture.test'

it('retains cyclic raw options and genuine context when a custom raw loader runs', async () => {
  // Given
  const request = fixture('@mdx-js/loader')
  const observations: unknown[] = []
  const configured: { self?: unknown; observe: (value: unknown) => void } = {
    observe: (value) => observations.push(value),
  }
  configured.self = configured
  const raw = join(request.root, 'raw.cjs')
  writeFileSync(
    raw,
    'module.exports = function() { const options = this.getOptions(); for (const value of [options, options.self, this._compiler, this.mode, this.sourceMap]) options.observe(value); return "# raw identity" }',
  )
  const pipeline = {
    ...request.pipeline,
    loaders: [
      ...request.pipeline.loaders,
      { loader: raw, options: configured },
    ],
  }
  // When
  const output = await compileMdx({ ...request, pipeline })
  // Then
  expect(observations).toEqual([
    configured,
    configured,
    request.context.compiler,
    'production',
    true,
  ])
  expect(observations[0]).toBe(configured)
  expect(observations[1]).toBe(configured)
  expect(observations[2]).toBe(request.context.compiler)
  expect(output.source).toContain('raw identity')
})

function plugin(text: string) {
  return () => (tree: RemarkTree) => {
    const node = tree.children[0]?.children[0]
    if (node) node.value = text
  }
}
it.each([true, false])(
  'isolates colliding function plugins when compiler and resource match and shared owner is %s',
  async (sharedOwner) => {
    // Given
    const a = fixture('@mdx-js/loader', { remarkPlugins: [plugin('app-a')] })
    const b = fixture('@mdx-js/loader', { remarkPlugins: [plugin('app-b')] })
    const first = await compileMdx(a)
    // When
    const second = await compileMdx({
      ...b,
      filename: a.filename,
      context: sharedOwner
        ? a.context
        : { ...b.context, compiler: a.context.compiler },
    })
    // Then
    expect(first.source).toContain('app-a')
    expect(second.source).toContain('app-b')
  },
)

it('uses updated live functions when the pipeline generation changes', async () => {
  // Given
  const options = { remarkPlugins: [plugin('old-generation')] }
  const request = fixture('@mdx-js/loader', options)
  await compileMdx(request)
  options.remarkPlugins[0] = plugin('new-generation')
  // When
  const output = await compileMdx({
    ...request,
    context: { ...request.context, generation: {} },
  })
  // Then
  expect(output.source).toContain('new-generation')
})

it('retains cache owners only when every semantic scope matches', () => {
  // Given
  const a = fixture('@mdx-js/loader')
  const b = fixture('@mdx-js/loader')
  const directory = dirname(a.filename)
  const owner = mdxPrewarmCacheOwner(a.context, a.pipeline, directory)
  // When / Then
  expect(mdxPrewarmCacheOwner(a.context, a.pipeline, directory)).toBe(owner)
  for (const context of [
    { ...a.context, owner: {} },
    { ...a.context, generation: {} },
    { ...a.context, compiler: {} },
    { ...a.context, mode: 'development' },
    { ...a.context, sourceMap: false },
  ] satisfies readonly MdxPreparationContext[])
    expect(mdxPrewarmCacheOwner(context, a.pipeline, directory)).not.toBe(owner)
  expect(mdxPrewarmCacheOwner(a.context, b.pipeline, directory)).not.toBe(owner)
  expect(
    mdxPrewarmCacheOwner(a.context, a.pipeline, dirname(b.filename)),
  ).not.toBe(owner)
})

it.each([true, false])(
  'retains genuine sourceMap %s and development mode in compiler output',
  async (sourceMap) => {
    // Given
    const request = fixture('@mdx-js/loader')
    await compileMdx(request)
    // When
    const output = await compileMdx({
      ...request,
      context: { ...request.context, mode: 'development', sourceMap },
    })
    // Then
    expect(output.source).toContain('jsx-dev-runtime')
    expect(output.map !== undefined).toBe(sourceMap)
  },
)
