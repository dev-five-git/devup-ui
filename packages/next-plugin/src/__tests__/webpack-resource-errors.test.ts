import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { describe, expect, it } from 'bun:test'
import type { Configuration } from 'webpack'

import { resolveWebpackLoader } from '../webpack-mdx-resolution'
import { nativeRuleSet } from '../webpack-resource-native'
import { collectResourceRules } from '../webpack-resource-rules'
import {
  extraction,
  mdxRule,
  source,
  withSelector,
} from './webpack-resource-fixture'

describe('resource boundary error contracts', () => {
  it('refuses an unavailable receiving RuleSet rather than substituting a local matcher', () => {
    // Given / When / Then
    expect(() => nativeRuleSet({})).toThrow(TypeError)
  })

  it('rejects nonboolean native loader source-map overrides', async () => {
    // Given
    const config: Configuration = {
      loader: { sourceMap: 'unproved' },
      module: { rules: [mdxRule()] },
    }
    // When / Then
    await expect(withSelector(config, () => true)).rejects.toBeInstanceOf(
      TypeError,
    )
  })

  it('keeps custom resolver callouts outside resource-only certification', async () => {
    // Given
    const config: Configuration = {
      resolveLoader: { plugins: [{ apply() {} }] },
      module: { rules: [mdxRule()] },
    }
    // When / Then
    await expect(
      withSelector(config, (selector) =>
        selector.selectPipeline(source, new AbortController().signal),
      ),
    ).rejects.toMatchObject({
      rulePosition: 'webpack.resolveLoader.plugins',
      unknownFact: 'loader resolver context',
    })
  })

  it('requires native ordinary input when no Devup pre boundary exists', async () => {
    // Given
    const ordinary = source.replace(/\.mdx$/, '.tsx')
    // When
    const result = await withSelector({ module: { rules: [] } }, (selector) =>
      selector.qualifyOrdinary(ordinary, new AbortController().signal),
    )
    // Then
    expect(result).toMatchObject({
      kind: 'native-required',
      unknownFact: 'first-input boundary',
    })
  })

  it('keeps an ordinary downstream use factory outside disk-first admission', async () => {
    // Given
    const ordinary = source.replace(/\.mdx$/, '.tsx')
    let calls = 0
    const config: Configuration = {
      module: {
        rules: [
          { test: /\.tsx$/, enforce: 'pre', use: [extraction] },
          {
            test: /\.tsx$/,
            use: function downstreamFactory() {
              calls += 1
              return []
            },
          },
        ],
      },
    }
    // When
    const result = await withSelector(config, (selector) =>
      selector.qualifyOrdinary(ordinary, new AbortController().signal),
    )
    // Then
    expect(result.kind).toBe('native-required')
    expect(calls).toBe(0)
  })

  it('collects shorthand loader options through the native resource envelope', async () => {
    // Given
    const options = { original: true }
    // When
    const result = await withSelector(
      {
        module: {
          rules: [{ test: /\.tsx$/, loader: 'ordinary-loader', options }],
        },
      },
      (_selector, _compiler, binding) => collectResourceRules(binding),
    )
    // Then
    expect(result.find((rule) => rule.key === '0')?.loaders).toEqual([
      { loader: 'ordinary-loader', options },
    ])
  })

  it('refuses loader files without native installed package-version facts', async () => {
    // Given
    const root = mkdtempSync(join(tmpdir(), 'w21f-resource-version-'))
    const loader = join(root, 'loader.cjs')
    writeFileSync(loader, 'module.exports = value => value')
    try {
      // When / Then
      await expect(
        withSelector(
          { context: root, module: { rules: [] } },
          (_selector, _compiler, binding) =>
            resolveWebpackLoader(
              binding,
              { loader },
              new AbortController().signal,
            ),
        ),
      ).rejects.toBeInstanceOf(TypeError)
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  })

  it('refuses a qualified pipeline when the real receiving factory selects another chain', async () => {
    // Given
    const config = {
      module: {
        rules: [
          { oneOf: [{ test: /\.mdx$/, type: 'javascript/auto' }, mdxRule()] },
        ],
      },
    }
    // When / Then
    await expect(
      withSelector(config, (selector) =>
        selector.selectPipeline(source, new AbortController().signal),
      ),
    ).rejects.toMatchObject({ unknownFact: 'selected native chain' })
  })

  it('rejects inline loader requests instead of presenting them as disk-only input', async () => {
    // Given
    const filename = `inline-loader!${source}`
    // When / Then
    await expect(
      withSelector({ module: { rules: [mdxRule()] } }, (selector) =>
        selector.selectPipeline(filename, new AbortController().signal),
      ),
    ).rejects.toMatchObject({ unknownFact: 'plain disk resource' })
  })
})
