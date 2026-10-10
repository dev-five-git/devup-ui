import { mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { compileMdx } from '../mdx-prepare'
import {
  fixture,
  originalCompile,
  type RemarkTree,
} from './mdx-boundary-fixture.test'

it.each([true, false])(
  'is byte equal to Next with all plugin forms when jsx is %s',
  async (jsx) => {
    // Given
    const request = fixture('@next/mdx/mdx-js-loader')
    const named = join(request.root, 'named.cjs')
    const tuple = join(request.root, 'tuple.cjs')
    writeFileSync(
      named,
      'module.exports = function() { return function(tree) { tree.children[0].children[0].value += " named" } }',
    )
    writeFileSync(
      tuple,
      'module.exports = function(options) { return function(tree) { tree.children[0].children[0].value += options.text } }',
    )
    const functionPlugin = () => (tree: RemarkTree) => {
      const text = tree.children[0]?.children[0]
      if (text) text.value += ' function'
    }
    const options = {
      jsx,
      providerImportSource: 'next-mdx-import-source-file',
      remarkPlugins: [functionPlugin, named, [tuple, { text: ' tuple' }]],
    }
    const configured = {
      ...request,
      pipeline: {
        ...request.pipeline,
        loaders: request.pipeline.loaders.map((loader) => ({
          ...loader,
          options,
        })),
      },
    }
    const nativeOptions = {
      ...options,
      remarkPlugins: [functionPlugin, named, [tuple, { text: ' tuple' }]],
    }
    const expected = await originalCompile({
      ...request,
      pipeline: {
        ...request.pipeline,
        loaders: request.pipeline.loaders.map((loader) => ({
          ...loader,
          options: nativeOptions,
        })),
      },
    })
    // When
    const output = await compileMdx(configured)
    // Then
    expect(output.source).toBe(expected.source)
    expect(output.map).toEqual(expected.map)
    expect(output.source).toContain('function named tuple')
    expect(output.source).toContain('next-mdx-import-source-file')
  },
)

it.each([true, false])(
  'is byte equal to the direct original chain when jsx is %s',
  async (jsx) => {
    // Given
    const plugin = () => (tree: RemarkTree) => {
      const text = tree.children[0]?.children[0]
      if (text) text.value = 'direct function output'
    }
    const request = fixture('@mdx-js/loader', {
      jsx,
      remarkPlugins: [plugin],
      providerImportSource: 'provider',
    })
    const expected = await originalCompile(request)
    // When
    const output = await compileMdx(request)
    // Then
    expect(output.source).toBe(expected.source)
    expect(output.map).toEqual(expected.map)
    expect(output.source).toContain('direct function output')
  },
)

it('uses resource-directory named plugins when identical names exist in different folders', async () => {
  // Given
  const request = fixture('@next/mdx/mdx-js-loader', {
    remarkPlugins: ['context-plugin'],
  })
  const parent = join(request.root, 'content')
  const nested = join(parent, 'nested')
  for (const directory of [parent, nested]) {
    const module = join(directory, 'node_modules/context-plugin')
    mkdirSync(module, { recursive: true })
    writeFileSync(join(module, 'package.json'), '{"main":"index.cjs"}')
    writeFileSync(
      join(module, 'index.cjs'),
      'module.exports = function() { return function(tree) { tree.children[0].children[0].value = "wrong parent" } }',
    )
  }
  const plugin = join(nested, 'node_modules/context-plugin/index.cjs')
  writeFileSync(
    plugin,
    'module.exports = function() { return function(tree) { tree.children[0].children[0].value = "resource-directory" } }',
  )
  const filename = join(nested, 'page.mdx')
  writeFileSync(filename, '# original')
  const configured = { ...request, filename }
  const expected = await originalCompile(configured)
  // When
  const output = await compileMdx(configured)
  // Then
  expect(output.source).toBe(expected.source)
  expect(output.source).toContain('resource-directory')
  expect(output.dependencies).toContain(plugin)
})
