import { fileURLToPath } from 'node:url'

import { describe, expect, it } from 'bun:test'
import type { Configuration } from 'webpack'

import {
  extraction,
  mdxLoader,
  mdxRule,
  source,
  withSelector,
} from './webpack-resource-fixture'

describe('initial native preparation gates', () => {
  it('keeps opaque external factories outside ordinary disk replay', async () => {
    // Given
    const ordinary = source.replace(/\.mdx$/, '.tsx')
    const config: Configuration = {
      externals: { opaque: 'opaque' },
      module: {
        rules: [{ test: /\.tsx$/, enforce: 'pre', use: [extraction] }],
      },
    }
    // When
    const result = await withSelector(config, (selector) =>
      selector.qualifyOrdinary(ordinary, new AbortController().signal),
    )
    // Then
    expect(result).toMatchObject({
      kind: 'native-required',
      rulePosition: 'webpack.externals',
    })
  })

  it('accepts a nonmatching empty plugin slot and exact string alias values', async () => {
    // Given
    const config: Configuration = {
      plugins: [false],
      resolve: { alias: [{ name: 'provider', alias: 'first' }] },
      module: { rules: [mdxRule()] },
    }
    // When
    const result = await withSelector(config, (selector) => selector.aliases)
    // Then
    expect(result).toEqual({ provider: ['first'] })
  })

  it('rejects a compiler whose native resolver substitutes an unrecognized compiler module', async () => {
    // Given
    const replacement = fileURLToPath(
      new URL('./webpack-resource-downstream.cjs', import.meta.url),
    )
    const config: Configuration = {
      resolveLoader: { alias: { [mdxLoader]: replacement } },
      module: { rules: [mdxRule()] },
    }
    // When / Then
    await expect(
      withSelector(config, (selector) =>
        selector.selectPipeline(source, new AbortController().signal),
      ),
    ).rejects.toMatchObject({ unknownFact: 'installed compiler semantics' })
  })

  it('reuses a captured native pipeline across resources within the receiving factory', async () => {
    // Given
    const config = { module: { rules: [mdxRule({ jsx: true })] } }
    // When
    const result = await withSelector(config, async (selector) => {
      const first = await selector.selectPipeline(
        source,
        new AbortController().signal,
      )
      const second = await selector.selectPipeline(
        source.replace('page.mdx', 'other.mdx'),
        new AbortController().signal,
      )
      return { first, second }
    })
    // Then
    expect(result.first?.pipeline).toBe(result.second?.pipeline)
    expect(result.first?.identity).toBe(result.second?.identity)
  })

  it('rejects custom MDX extensions while SourceType7f is unannounced', async () => {
    // Given
    const config = { module: { rules: [{ ...mdxRule(), test: /\.mdown$/ }] } }
    // When / Then
    await expect(
      withSelector(config, (selector) =>
        selector.selectPipeline(
          source.replace(/\.mdx$/, '.mdown'),
          new AbortController().signal,
        ),
      ),
    ).rejects.toMatchObject({
      unknownFact: 'extraction source type',
      boundary: {
        reason: expect.stringMatching(/extension \.mdown.*use \.md or \.mdx/),
      },
    })
  })

  it('admits markdown through the original native MDX compiler without renaming its resource', async () => {
    // Given
    const filename = source.replace(/\.mdx$/, '.md')
    const options = { jsx: true }
    const config = {
      module: { rules: [{ ...mdxRule(options), test: /\.md$/ }] },
    }
    // When
    const selection = await withSelector(config, (selector) =>
      selector.selectPipeline(filename, new AbortController().signal),
    )
    // Then
    expect(selection?.loaders[0]?.options).toBe(options)
    expect(selection?.pipeline.ruleKey).toBe('0')
  })
})
