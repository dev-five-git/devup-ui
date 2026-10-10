import { describe, expect, it } from 'bun:test'

import {
  composeMdxRules,
  MdxPipelineError,
  requireMdxPipeline,
} from '../mdx-pipeline'

const compiler = { loader: '@next/mdx/mdx-js-loader', options: { jsx: true } }
const extraction = { loader: '/devup/extract.cjs', options: { token: 'app' } }

describe('MDX pipeline composition', () => {
  it('preserves arbitrary Turbo keys and unrelated entries when composing', () => {
    // Given
    const unrelated = { loaders: ['image-loader'], as: '*.js' }
    const condition = { all: [{ path: /\.mdx$/ }, 'browser'] }
    const aliases = { provider: ['src/provider', 'fallback'] }
    const rule = { condition, loaders: [compiler, 'raw-loader'], as: '*.tsx' }
    const rules = { arbitrary: [unrelated, rule], '*.png': unrelated }
    // When
    const result = composeMdxRules(
      { bundler: 'turbo', rules, aliases },
      extraction,
    )
    // Then
    expect(result.rules).toEqual({
      arbitrary: [
        unrelated,
        { ...rule, loaders: [extraction, compiler, 'raw-loader'] },
      ],
      '*.png': unrelated,
    })
    expect(result.pipelines[0]).toMatchObject({
      loaders: [compiler, { loader: 'raw-loader' }],
      aliases,
    })
    expect(result.pipelines[0]?.conditions).toEqual([rule])
    expect(rule.loaders).toEqual([compiler, 'raw-loader'])
  })

  it('composes nested webpack chains before the compiler when downstream loaders exist', () => {
    // Given
    const rule = {
      test: /\.mdx$/,
      use: ['next-swc-loader', extraction, compiler, 'raw'],
    }
    const parent = { issuer: /page/, oneOf: [rule, { use: ['css'] }] }
    // When
    const result = composeMdxRules(
      { bundler: 'webpack', rules: [parent], aliases: {} },
      extraction,
    )
    // Then
    expect(result.rules).toEqual([parent])
    expect(result.pipelines[0]?.loaders).toEqual([compiler, { loader: 'raw' }])
    expect(result.pipelines[0]?.conditions).toEqual([parent, rule])
  })

  it('reports the actual required module when its chain is unavailable', () => {
    // Given
    const filename = '/app/content/page.mdx'
    // When / Then
    expect(() => requireMdxPipeline(filename, undefined)).toThrow(
      `${filename}:1:1:`,
    )
  })

  it('inserts extraction once when webpack has nested rules and no previous extractor', () => {
    // Given
    const rule = {
      use: ['next-flight-loader', 'next-swc-loader', compiler],
      include: '/app',
    }
    // When
    const result = composeMdxRules(
      { bundler: 'webpack', rules: [{ rules: [rule] }, false], aliases: {} },
      extraction,
    )
    // Then
    expect(result.rules).toEqual([
      {
        rules: [
          {
            ...rule,
            use: [
              'next-flight-loader',
              'next-swc-loader',
              extraction,
              compiler,
            ],
          },
        ],
      },
      false,
    ])
    expect(
      requireMdxPipeline('/real.mdx', result.pipelines[0]).loaders,
    ).toEqual([compiler])
  })

  it.each([
    [compiler, { loader: 'raw', options: 3 }],
    [compiler, { loader: 'raw', ident: 3 }],
    [compiler, false],
    [compiler, 'next-swc-loader'],
    [compiler, 'next-flight-loader'],
    [compiler, compiler],
  ])(
    'blocks required malformed or downstream compiler segments when chain is %j',
    (...loaders) => {
      // Given
      const rule = { loaders }
      // When
      const result = composeMdxRules(
        { bundler: 'turbo', rules: { '*': rule }, aliases: {} },
        extraction,
      )
      // Then
      expect(result.rules).toEqual({ '*': rule })
      expect(() =>
        requireMdxPipeline('/real.mdx', result.pipelines[0]),
      ).toThrow(MdxPipelineError)
    },
  )

  it('leaves unrecognized and non-array rules alone when no verified compiler exists', () => {
    // Given
    const rules = {
      '*': [
        { loaders: ['@next/mdx/mdx-rs-loader'] },
        { loaders: [{ loader: 'other', options: '?key=value', ident: 'id' }] },
        'ignore',
      ],
      '*.md': { loaders: 'dynamic' },
    }
    // When
    const result = composeMdxRules(
      { bundler: 'turbo', rules, aliases: {} },
      extraction,
    )
    // Then
    expect(result).toEqual({ rules, pipelines: [] })
  })

  it('recognizes direct MDX loader requests when compiler options use strings', () => {
    // Given
    const compiler = {
      loader: '@mdx-js/loader',
      options: '?jsx=true',
      ident: 'configured',
    }
    // When
    const result = composeMdxRules(
      {
        bundler: 'turbo',
        rules: { '*': { loaders: [compiler] } },
        aliases: {},
      },
      extraction,
    )
    // Then
    expect(result.pipelines[0]?.loaders).toEqual([compiler])
  })

  it('relocates duplicate extraction loaders when they were inserted before raw compilation', () => {
    // Given
    const rule = { loaders: [extraction, compiler, extraction, 'raw'] }
    // When
    const result = composeMdxRules(
      { bundler: 'turbo', rules: { '*': rule }, aliases: {} },
      extraction,
    )
    // Then
    expect(result.rules).toEqual({
      '*': { loaders: [extraction, compiler, 'raw'] },
    })
    expect(result.pipelines[0]?.loaders).toEqual([compiler, { loader: 'raw' }])
  })
})
