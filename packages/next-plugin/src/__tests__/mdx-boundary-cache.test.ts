import { writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { admitMdxNativeCell } from '../mdx-native-scope'
import { compileMdx, MdxCompileError } from '../mdx-prepare'
import { fixture, type RemarkTree } from './mdx-boundary-fixture.test'

it('retains cyclic raw options and finite scalar context when a custom raw loader runs', async () => {
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
    'module.exports = function() { const options = this.getOptions(); for (const value of [options, options.self, this.mode, this.sourceMap]) options.observe(value); return "# raw identity" }',
  )
  const pipeline = {
    ...request.pipeline,
    loaders: [
      ...request.pipeline.loaders,
      { loader: raw, options: configured, ident: 'caller-cyclic-raw' },
    ],
  }
  // When
  const output = await compileMdx({ ...request, pipeline })
  // Then
  expect(observations).toEqual([configured, configured, 'production', true])
  expect(observations[0]).toBe(configured)
  expect(observations[1]).toBe(configured)
  expect(output.source).toContain('raw identity')
})

function plugin(text: string) {
  return () => (tree: RemarkTree) => {
    const node = tree.children[0]?.children[0]
    if (node) node.value = text
  }
}
it.each([true, false])(
  'refuses colliding function plugins when compiler and resource match and shared owner is %s',
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
    }).catch((error: unknown) => error)
    // Then
    expect(first.source).toContain('app-a')
    expect(second).toBeInstanceOf(MdxCompileError)
  },
)

it('refuses updated live functions in the same native cell when the pipeline generation changes', async () => {
  // Given
  const options = { remarkPlugins: [plugin('old-generation')] }
  const request = fixture('@mdx-js/loader', options)
  await compileMdx(request)
  options.remarkPlugins[0] = plugin('new-generation')
  // When
  const output = await compileMdx({
    ...request,
    context: { ...request.context, generation: {} },
  }).catch((error: unknown) => error)
  // Then
  expect(output).toBeInstanceOf(MdxCompileError)
})

it('retains private cell tokens across structural contexts but not native module or Compiler identity', () => {
  // Given
  const module = {}
  const compiler = {}
  const effective = { options: {}, mode: 'production', sourceMap: true }
  const owner = admitMdxNativeCell(module, compiler, effective).compiler
  // When / Then
  expect(
    admitMdxNativeCell(module, compiler, { ...effective, options: {} })
      .compiler,
  ).toBe(owner)
  expect(admitMdxNativeCell({}, compiler, effective).compiler).not.toBe(owner)
  expect(admitMdxNativeCell(module, {}, effective).compiler).not.toBe(owner)
})

it.each([true, false])(
  'refuses incompatible sourceMap %s and development mode in an already admitted native cell',
  async (sourceMap) => {
    // Given
    const request = fixture('@mdx-js/loader')
    await compileMdx(request)
    // When
    const output = await compileMdx({
      ...request,
      context: { ...request.context, mode: 'development', sourceMap },
    }).catch((error: unknown) => error)
    // Then
    expect(output).toBeInstanceOf(MdxCompileError)
  },
)
