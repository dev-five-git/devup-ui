import { fileURLToPath } from 'node:url'

import { describe, expect, it } from 'bun:test'

import { WebpackResourceError } from '../webpack-resource-error'
import {
  extraction,
  mdxRule,
  source,
  withSelector,
} from './webpack-resource-fixture'

describe('native resource-invariant initial selection', () => {
  it('retains the real receiving compiler when downstream layer conditions change', async () => {
    // Given
    const config = {
      experiments: { layers: true },
      module: {
        rules: [
          {
            issuerLayer: 'rsc',
            use: [
              fileURLToPath(
                new URL('./webpack-resource-downstream.cjs', import.meta.url),
              ),
            ],
          },
          {
            issuer: /unobserved/,
            resourceQuery: /unknown/,
            dependency: 'url',
            use: [],
          },
          mdxRule({ jsx: true }),
        ],
      },
    }
    // When
    const result = await withSelector(config, async (selector, compiler) => {
      const selection = await selector.selectPipeline(
        source,
        new AbortController().signal,
      )
      return { compiler, selection }
    })
    // Then
    expect(result.selection?.context.compiler).toBe(result.compiler)
    expect(result.selection?.pipeline.loaders).toHaveLength(1)
    expect(result.selection?.loaders[0]?.options).toEqual({ jsx: true })
  })

  it('rejects future source-reader callouts despite identical current native RuleSet effects', async () => {
    // Given
    const config = {
      module: { rules: [mdxRule()] },
      plugins: [
        {
          apply(compiler: import('webpack').Compiler) {
            compiler.hooks.compilation.tap(
              'SourceReaderSubstitution',
              (compilation) => {
                compiler.webpack.NormalModule.getCompilationHooks(compilation)
                  .readResource.for(undefined)
                  .tapAsync('SourceReaderSubstitution', (_context, callback) =>
                    callback(null, '# Different input'),
                  )
              },
            )
          },
        },
      ],
    }
    // When / Then
    await expect(
      withSelector(config, (selector) =>
        selector.selectPipeline(source, new AbortController().signal),
      ),
    ).rejects.toMatchObject({
      rulePosition: 'webpack.plugins[0]',
      unknownFact: 'pitch/source-reader/factory substitution',
    })
  })

  it('rejects a contextual global pre-loader that can change MDX input', async () => {
    // Given
    const config = {
      module: {
        rules: [
          mdxRule(),
          {
            test: /\.mdx$/,
            issuer: /special/,
            enforce: 'pre' as const,
            use: ['unresolved-input-loader'],
          },
        ],
      },
    }
    // When / Then
    await expect(
      withSelector(config, (selector) =>
        selector.selectPipeline(source, new AbortController().signal),
      ),
    ).rejects.toMatchObject({
      name: 'WebpackResourceError',
      rulePosition: 'module.rules[1]',
      unknownFact: 'issuer',
    })
  })

  it('rejects a competing contextual MDX segment instead of selecting the first pipeline', async () => {
    // Given
    const conditional = { ...mdxRule({ jsx: true }), issuerLayer: 'rsc' }
    // When / Then
    await expect(
      withSelector(
        { module: { rules: [mdxRule(), conditional] } },
        (selector) =>
          selector.selectPipeline(source, new AbortController().signal),
      ),
    ).rejects.toBeInstanceOf(WebpackResourceError)
  })

  it('proves ordinary first-input from native order rather than a pre suffix', async () => {
    // Given
    const ordinary = source.replace(/\.mdx$/, '.tsx')
    const config = {
      module: {
        rules: [
          { test: /\.tsx$/, enforce: 'pre' as const, use: [extraction] },
          {
            test: /\.tsx$/,
            issuer: /hidden/,
            enforce: 'pre' as const,
            use: ['unresolved-input-loader'],
          },
        ],
      },
    }
    // When
    const eligibility = await withSelector(config, (selector) =>
      selector.ordinaryEligibility(ordinary),
    )
    // Then
    expect(eligibility).toMatchObject({
      kind: 'native-required',
      unknownFact: 'issuer',
    })
    expect('proof' in eligibility).toBe(false)
  })

  it('admits an unrelated pre-loader without a local rule matcher', async () => {
    // Given
    const config = {
      module: {
        rules: [
          {
            test: /\.png$/,
            issuer: /hidden/,
            enforce: 'pre' as const,
            use: ['unresolved-input-loader'],
          },
          mdxRule(),
        ],
      },
    }
    // When
    const selection = await withSelector(config, (selector) =>
      selector.selectPipeline(source, new AbortController().signal),
    )
    // Then
    expect(selection?.pipeline.loaders).toHaveLength(1)
  })
})
