import { expect, it } from 'bun:test'

import { compileMdx, MdxCompileError } from '../mdx-prepare'
import { fixture, type RemarkTree } from './mdx-boundary-fixture.test'
import { withNativeMdx } from './mdx-safety-native'

const plugin = (text: string) => () => (tree: RemarkTree) => {
  const node = tree.children[0]?.children[0]
  if (node) node.value = text
}

it.each(['@next/mdx/mdx-js-loader', '@mdx-js/loader'])(
  'retains opaque compiler option identity without inventing a native partition for %s',
  async (entry) => {
    // Given
    const leaf = new Map([['text', 'opaque-map']])
    const seen: unknown[] = []
    const attacher = (options: Map<string, string>) => {
      seen.push(options)
      return (tree: RemarkTree) => {
        const node = tree.children[0]?.children[0]
        if (node) node.value = options.get('text') ?? ''
      }
    }
    const request = fixture(entry, {
      format: 'mdx',
      remarkPlugins: [[attacher, leaf]],
    })
    await withNativeMdx(request, async (native) => {
      const expected = await native.compile()
      // When
      const output = await compileMdx({
        ...request,
        context: { ...request.context, compiler: native.compiler },
      })
      // Then
      expect(output.source).toBe(expected.source)
      expect(seen).toHaveLength(2)
      expect(seen[0]).toBe(leaf)
      expect(seen[1]).toBe(leaf)
    })
  },
)

it.each(['@next/mdx/mdx-js-loader', '@mdx-js/loader'])(
  'proves native function collision before requiring located private refusal for %s',
  async (entry) => {
    // Given: actual webpack keeps one Compiler alive across two different live functions.
    const a = { format: 'mdx', remarkPlugins: [plugin('cell-a')] }
    const b = { format: 'mdx', remarkPlugins: [plugin('cell-b')] }
    const request = fixture(entry, a)
    await withNativeMdx(request, async (native) => {
      const first = await native.compile(a)
      const second = await native.compile(b)
      expect(first.source).toContain('cell-a')
      expect(second.source).toBe(first.source)
      const context = { ...request.context, compiler: native.compiler }
      await compileMdx({ ...request, context })
      // When
      const result = await compileMdx({
        ...request,
        context,
        pipeline: {
          ...request.pipeline,
          loaders: request.pipeline.loaders.map((loader) => ({
            ...loader,
            options: b,
          })),
        },
      }).catch((error: unknown) => error)
      // Then
      expect(result).toBeInstanceOf(MdxCompileError)
      expect(result).toMatchObject({ filename: request.filename })
      if (!(result instanceof Error))
        throw new TypeError('missing collision error')
      expect(result.message).toContain('remarkPlugins')
    })
  },
)

it.each(
  ['@next/mdx/mdx-js-loader', '@mdx-js/loader'].flatMap((entry) =>
    [true, false].map((privateFirst) => ({ entry, privateFirst })),
  ),
)(
  'keeps native/private priming separate for $entry privateFirst=$privateFirst',
  async ({ entry, privateFirst }) => {
    // Given
    const a = { format: 'mdx', remarkPlugins: [plugin('private-cell')] }
    const b = { format: 'mdx', remarkPlugins: [plugin('native-cell')] }
    const request = fixture(entry, a)
    await withNativeMdx(request, async (native) => {
      const context = { ...request.context, compiler: native.compiler }
      if (privateFirst) await compileMdx({ ...request, context })
      else await native.compile(b)
      // When
      const privateOutput = await compileMdx({ ...request, context })
      const nativeOutput = await native.compile(b)
      // Then
      expect(privateOutput.source).toContain('private-cell')
      expect(nativeOutput.source).toContain('native-cell')
    })
  },
)

it.each([true, false])(
  'proves native effective mode/maps collision before private refusal for maps=%s',
  async (sourceMap) => {
    // Given
    const request = fixture('@mdx-js/loader', { format: 'mdx' })
    await withNativeMdx(request, async (native) => {
      const first = await native.compile()
      const second = await native.compile(undefined, {
        mode: 'development',
        sourceMap,
      })
      expect(second.source).toBe(first.source)
      expect(second.map).toEqual(first.map)
      const context = { ...request.context, compiler: native.compiler }
      await compileMdx({ ...request, context })
      // When
      const output = await compileMdx({
        ...request,
        context: { ...context, mode: 'development', sourceMap },
      }).catch((error: unknown) => error)
      // Then
      expect(output).toBeInstanceOf(MdxCompileError)
    })
  },
)

it.each(
  ['@next/mdx/mdx-js-loader', '@mdx-js/loader'].flatMap((entry) =>
    [true, false].map((sourceMap) => ({ entry, sourceMap })),
  ),
)(
  'matches native bytes/maps and reuses exactly one private fixed-format cell for $entry maps=$sourceMap',
  async ({ entry, sourceMap }) => {
    // Given
    let constructions = 0
    const attacher = () => {
      constructions++
      return () => undefined
    }
    const request = fixture(entry, { format: 'mdx', remarkPlugins: [attacher] })
    const configured = {
      ...request,
      context: { ...request.context, sourceMap },
    }
    await withNativeMdx(configured, async (native) => {
      const expected = await native.compile()
      const nativeCount = constructions
      const context = { ...configured.context, compiler: native.compiler }
      // When
      const first = await compileMdx({ ...configured, context })
      const afterFirst = constructions
      const second = await compileMdx({
        ...configured,
        context: { ...context, owner: {}, generation: {} },
      })
      // Then
      expect(first.source).toBe(expected.source)
      expect(first.map).toEqual(expected.map)
      expect(second.source).toBe(first.source)
      expect(constructions).toBe(afterFirst)
      expect(afterFirst - nativeCount).toBe(nativeCount)
      expect(nativeCount).toBeGreaterThan(0)
      const repeated = await native.compile()
      expect(repeated.source).toBe(expected.source)
      expect(constructions).toBe(afterFirst)
    })
  },
)
