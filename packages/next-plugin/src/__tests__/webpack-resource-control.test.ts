import { describe, expect, it } from 'bun:test'

import {
  extraction,
  mdxRule,
  source,
  withSelector,
} from './webpack-resource-fixture'

describe('native oneOf control dependence', () => {
  it('rejects an unknown earlier branch even when that branch has no loader effect', async () => {
    // Given
    const config = {
      module: {
        rules: [
          {
            oneOf: [{ issuer: /special/, type: 'javascript/auto' }, mdxRule()],
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
      unknownFact: 'issuer',
      rulePosition: 'module.rules[0].oneOf[0]',
    })
  })

  it('keeps an ordinary boundary native-required when an earlier branch can hide Devup', async () => {
    // Given
    const ordinary = source.replace(/\.mdx$/, '.tsx')
    const config = {
      module: {
        rules: [
          {
            oneOf: [
              { resourceQuery: /special/, type: 'javascript/auto' },
              { test: /\.tsx$/, enforce: 'pre' as const, use: [extraction] },
            ],
          },
        ],
      },
    }
    // When
    const result = await withSelector(config, (selector) =>
      selector.qualifyOrdinary(ordinary, new AbortController().signal),
    )
    // Then
    expect(result).toMatchObject({
      kind: 'native-required',
      unknownFact: 'resourceQuery',
    })
  })

  it('checks actual native ordinary selection when a known earlier branch hides Devup', async () => {
    // Given
    const ordinary = source.replace(/\.mdx$/, '.tsx')
    const config = {
      module: {
        rules: [
          {
            oneOf: [
              { test: /\.tsx$/, type: 'javascript/auto' },
              { test: /\.tsx$/, enforce: 'pre' as const, use: [extraction] },
            ],
          },
        ],
      },
    }
    // When
    const result = await withSelector(config, (selector) =>
      selector.qualifyOrdinary(ordinary, new AbortController().signal),
    )
    // Then
    expect(result).toMatchObject({
      kind: 'native-required',
      unknownFact: 'selected first-input boundary',
    })
  })
})
