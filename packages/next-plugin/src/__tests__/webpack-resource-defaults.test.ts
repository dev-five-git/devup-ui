import { describe, expect, it } from 'bun:test'

import { mdxRule, source, withSelector } from './webpack-resource-fixture'

describe('receiving native default-rule input boundaries', () => {
  it('rejects an origin-dependent upstream loader from normalized defaultRules', async () => {
    // Given
    const config = {
      module: {
        defaultRules: [
          {
            test: /\.mdx$/,
            issuer: /hidden/,
            enforce: 'pre' as const,
            use: ['unresolved-default-input-loader'],
          },
        ],
        rules: [mdxRule()],
      },
    }
    // When / Then
    await expect(
      withSelector(config, (selector) =>
        selector.selectPipeline(source, new AbortController().signal),
      ),
    ).rejects.toMatchObject({
      rulePosition: 'module.defaultRules[0]',
      unknownFact: 'issuer',
    })
  })
})
