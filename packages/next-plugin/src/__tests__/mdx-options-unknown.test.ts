import { mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { createMdxOptionsInstance } from '../mdx-options-instance'
import { compileMdx } from '../mdx-prepare'
import { fixture } from './mdx-boundary-fixture.test'

it.each(['16.3.6', '16.3.7'])(
  'detaches mutable shells even when an original wrapper is unrecognized at version %s',
  async (version) => {
    // Given
    const request = fixture('@next/mdx/mdx-js-loader')
    const directory = join(request.root, '@next/mdx')
    mkdirSync(directory, { recursive: true })
    writeFileSync(join(directory, 'package.json'), JSON.stringify({ version }))
    const loader = join(directory, 'mdx-js-loader.js')
    writeFileSync(
      loader,
      'module.exports = function() { const options = this.getOptions(); options.remarkPlugins[0][0] = "resolved"; options.remarkPlugins.push("added"); options.other.count++; return "original wrapper " + options.other.count }',
    )
    const shared = { count: 0 }
    const tuple = ['configured', shared]
    const plugins = [tuple]
    const options = { remarkPlugins: plugins, other: shared }
    const pipeline = { ...request.pipeline, loaders: [{ loader, options }] }
    // When
    const output = await compileMdx({ ...request, pipeline })
    // Then
    expect(output.source).toBe('original wrapper 1')
    expect(options.remarkPlugins).toBe(plugins)
    expect(plugins).toEqual([['configured', shared]])
    expect(plugins[0]).toBe(tuple)
    expect(shared.count).toBe(1)
    expect(options.other).toBe(shared)
  },
)

it('keeps malformed mapped values and unrelated properties unchanged without adding defaults', () => {
  // Given
  const options = { remarkPlugins: 'malformed', other: {} }
  const request = fixture('@next/mdx/mdx-js-loader', options)
  // When
  const instance = createMdxOptionsInstance().loadersFor(request.pipeline)
  // Then
  expect(instance[0]?.options).toEqual(options)
  expect(instance[0]?.options).not.toBe(options)
})
