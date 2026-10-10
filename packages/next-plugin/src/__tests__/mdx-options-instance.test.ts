import { writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { createMdxOptionsInstance } from '../mdx-options-instance'
import { isMdxRecord, type MdxLoader } from '../mdx-pipeline'
import { compileMdx } from '../mdx-prepare'
import {
  fixture,
  originalCompile,
  type RemarkTree,
} from './mdx-boundary-fixture.test'

it.each(['webpack', 'turbo'] as const)(
  'keeps config containers and serialized bytes unchanged across files with %s',
  async (bundler) => {
    // Given
    const request = fixture('@next/mdx/mdx-js-loader')
    const named = join(request.root, 'named.cjs')
    const tuplePlugin = join(request.root, 'tuple.cjs')
    writeFileSync(
      named,
      'module.exports = function(options) { return function(tree) { tree.children[0].children[0].value += options ? options.text : " named" } }',
    )
    writeFileSync(
      tuplePlugin,
      'module.exports = function(options) { return function(tree) { tree.children[0].children[0].value += options.text } }',
    )
    const pluginOptions = { text: ' tuple', nested: { enabled: true } }
    const tuple = [tuplePlugin, pluginOptions]
    const array = [named, tuple]
    const options = {
      jsx: true,
      remarkPlugins: array,
      rehypePlugins: [],
      recmaPlugins: [],
      other: { stable: true },
    }
    const references = {
      rehype: options.rehypePlugins,
      recma: options.recmaPlugins,
      other: options.other,
      nested: pluginOptions.nested,
    }
    const loader = {
      loader: request.pipeline.loaders[0]?.loader ?? '',
      options,
    }
    const config = {
      turbopack: { rules: { '*.mdx': { loaders: [loader], as: '*.tsx' } } },
    }
    const bytes = JSON.stringify(config)
    const snapshot: unknown = JSON.parse(bytes)
    const pipeline = { ...request.pipeline, bundler, loaders: [loader] }
    const optionsInstance = createMdxOptionsInstance()
    const second = join(request.root, 'second.mdx')
    writeFileSync(second, '# second')
    // When
    const firstOutput = await compileMdx({
      ...request,
      pipeline,
      optionsInstance,
    })
    const secondOutput = await compileMdx({
      ...request,
      filename: second,
      pipeline,
      optionsInstance,
    })
    // Then
    expect(firstOutput.source).toContain('original named tuple')
    expect(secondOutput.source).toContain('second named tuple')
    expect(snapshot).toEqual(config)
    expect(JSON.stringify(config)).toBe(bytes)
    expect(config.turbopack.rules['*.mdx'].loaders[0]).toBe(loader)
    expect(loader.options).toBe(options)
    expect(options.remarkPlugins).toBe(array)
    expect(array[1]).toBe(tuple)
    expect(tuple[0]).toBe(tuplePlugin)
    expect(tuple[1]).toBe(pluginOptions)
    expect(options.rehypePlugins).toBe(references.rehype)
    expect(options.recmaPlugins).toBe(references.recma)
    expect(options.other).toBe(references.other)
    expect(pluginOptions.nested).toBe(references.nested)
    const instance = optionsInstance.loadersFor(pipeline)
    expect(optionsInstance.loadersFor(pipeline)).toBe(instance)
    const instanceOptions = instance[0]?.options
    if (
      !isMdxRecord(instanceOptions) ||
      !Array.isArray(instanceOptions.remarkPlugins)
    )
      throw new TypeError('missing instance options')
    expect(instanceOptions).not.toBe(options)
    expect(instanceOptions.remarkPlugins).not.toBe(array)
    expect(instanceOptions.remarkPlugins[1]).not.toBe(tuple)
    expect(typeof instanceOptions.remarkPlugins[1][0]).toBe('function')
    const freshOptions =
      createMdxOptionsInstance().loadersFor(pipeline)[0]?.options
    if (
      !isMdxRecord(freshOptions) ||
      !Array.isArray(freshOptions.remarkPlugins)
    )
      throw new TypeError('missing fresh options')
    expect(freshOptions.remarkPlugins[1][0]).toBe(tuplePlugin)
    expect(freshOptions).not.toBe(instanceOptions)
  },
)

it('copies only webpack compiler shells when other values and raw loader options are shared', () => {
  // Given
  const callback = () => undefined
  const shared = { callback }
  const tuple = [callback, shared]
  const options = {
    remarkPlugins: [callback, tuple],
    rehypePlugins: [tuple],
    recmaPlugins: [tuple],
    other: shared,
    callback,
  }
  const request = fixture('@next/mdx/mdx-js-loader', options)
  const raw = { loader: 'raw', options: shared }
  const pipeline = {
    ...request.pipeline,
    loaders: [...request.pipeline.loaders, raw],
  }
  // When
  const loaders = createMdxOptionsInstance().loadersFor(pipeline)
  // Then
  const got = loaders[0]?.options
  if (!isMdxRecord(got)) throw new TypeError('missing compiler options')
  expect(got.other).toBe(shared)
  expect(got.callback).toBe(callback)
  expect(loaders[1]).toBe(raw)
  for (const key of ['remarkPlugins', 'rehypePlugins', 'recmaPlugins']) {
    const plugins: unknown = got[key]
    if (!Array.isArray(plugins)) throw new TypeError('missing plugin array')
    const copiedTuple = plugins.find(Array.isArray)
    expect(copiedTuple).not.toBe(tuple)
    expect(copiedTuple?.[0]).toBe(callback)
    expect(copiedTuple?.[1]).toBe(shared)
  }
})

it('retains plugin-owned option mutations when webpack preparation processes more files', async () => {
  // Given
  const options = { count: 0 }
  const observations: unknown[] = []
  const plugin = (received: typeof options) => {
    observations.push(received)
    return (tree: RemarkTree) => {
      received.count += 1
      const text = tree.children[0]?.children[0]
      if (text) text.value = `visit-${received.count}`
    }
  }
  const request = fixture('@next/mdx/mdx-js-loader', {
    remarkPlugins: [[plugin, options]],
  })
  const optionsInstance = createMdxOptionsInstance()
  await compileMdx({ ...request, optionsInstance })
  // When
  const output = await compileMdx({ ...request, optionsInstance })
  // Then
  expect(output.source).toContain('visit-2')
  expect(options.count).toBe(2)
  expect(observations.every((value) => value === options)).toBe(true)
})

it.each(
  (['webpack', 'turbo'] as const).flatMap((bundler) =>
    [true, false].map((jsx) => ({ bundler, jsx })),
  ),
)(
  'is byte equal to the real original chain with strings and tuples for $bundler and JSX $jsx',
  async ({ bundler, jsx }) => {
    // Given
    const request = fixture('@next/mdx/mdx-js-loader')
    const named = join(request.root, 'named.cjs')
    writeFileSync(
      named,
      'module.exports = function(options) { return function(tree) { tree.children[0].children[0].value += options ? options.text : " named" } }',
    )
    const tuple = join(request.root, 'tuple.cjs')
    writeFileSync(
      tuple,
      'module.exports = function(options) { return function(tree) { tree.children[0].children[0].value += options.text } }',
    )
    const options = {
      jsx,
      providerImportSource: 'provider',
      remarkPlugins: [named, [tuple, { text: ' tuple' }]],
    }
    const configured = {
      ...request,
      pipeline: {
        ...request.pipeline,
        bundler,
        loaders: request.pipeline.loaders.map((loader) => ({
          ...loader,
          options,
        })),
      },
    }
    const nativeOptions: unknown = JSON.parse(JSON.stringify(options))
    if (!isMdxRecord(nativeOptions))
      throw new TypeError('invalid native options')
    const expected = await originalCompile({
      ...request,
      pipeline: {
        ...request.pipeline,
        loaders: request.pipeline.loaders.map((loader): MdxLoader => ({
          ...loader,
          options: nativeOptions,
        })),
      },
    })
    // When
    const output = await compileMdx({
      ...configured,
      optionsInstance: createMdxOptionsInstance(),
    })
    // Then
    expect(output.source).toBe(expected.source)
    expect(output.map).toEqual(expected.map)
    expect(output.source).toContain('original named tuple')
    expect(options.remarkPlugins[1]).toEqual([tuple, { text: ' tuple' }])
  },
)
