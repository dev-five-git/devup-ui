import { expect, it } from 'bun:test'
import type { Configuration } from 'webpack'

import { composeWebpackMdxRules } from '../webpack-coordinator-rules'

const extraction = {
  loader: '@devup-ui/next-plugin/loader',
  options: { token: 'same' },
}
const mdx = { loader: '@next/mdx/mdx-js-loader', options: { jsx: false } }

it('relocates duplicate extractors while retaining compiler options and unrelated functions', () => {
  // Given
  const customLoader = () => 'custom-loader'
  const config: Configuration = {
    resolve: {
      alias: [
        { name: 'provider', alias: '/provider.js', onlyModule: true },
        { name: 'other', alias: false },
      ],
    },
    module: {
      rules: [
        null,
        undefined,
        '',
        0,
        false,
        {
          test: /\.mdx$/,
          use: [customLoader, extraction.loader, mdx, extraction, 'raw-loader'],
        },
        { loader: 'other' },
      ],
    },
  }
  // When
  const result = composeWebpackMdxRules(config, extraction)
  // Then
  expect(result.rules[5]).toEqual({
    test: /\.mdx$/,
    use: [customLoader, extraction, mdx, 'raw-loader'],
  })
  expect(result.pipelines[0]?.aliases).toEqual({
    provider$: '/provider.js',
    other: false,
  })
  expect(config.module?.rules?.[5]).toEqual({
    test: /\.mdx$/,
    use: [customLoader, extraction.loader, mdx, extraction, 'raw-loader'],
  })
})

it('leaves malformed required chains for the lead preparation diagnostic', () => {
  // Given
  const rules = [{ use: [mdx, null] }, { use: [mdx, 'next-swc-loader'] }]
  // When
  const result = composeWebpackMdxRules({ module: { rules } }, extraction)
  // Then
  expect(result.rules).toEqual(rules)
  expect(
    result.pipelines.every((pipeline) => pipeline.issue !== undefined),
  ).toBe(true)
})
