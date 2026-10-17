import { fileURLToPath } from 'node:url'

import { describe, expect, it } from 'bun:test'
import type { Configuration } from 'webpack'

import {
  compareMdxBindingValues,
  snapshotMdxBindingValue,
} from '../mdx-binding-value'
import {
  nativeLoaders,
  nativeRuleSet,
  resourceFacts,
} from '../webpack-resource-native'
import {
  extraction,
  mdxRule,
  source,
  withSelector,
} from './webpack-resource-fixture'

describe('native resource boundary proofs', () => {
  it('admits an ordinary disk-first boundary only after async qualification', async () => {
    // Given
    const ordinary = source.replace(/\.mdx$/, '.tsx')
    const config = {
      module: {
        rules: [{ test: /\.tsx$/, enforce: 'pre' as const, use: [extraction] }],
      },
    }
    // When
    const result = await withSelector(config, async (selector) => ({
      initial: selector.ordinaryEligibility(ordinary),
      qualified: await selector.qualifyOrdinary(
        ordinary,
        new AbortController().signal,
      ),
      delivered: selector.ordinaryEligibility(ordinary),
    }))
    // Then
    expect(result.initial.kind).toBe('native-required')
    expect(result.qualified.kind).toBe('disk-first')
    expect(result.delivered).toBe(result.qualified)
  })

  it('keeps ordinary contextual input pending without issuing an attempt proof', async () => {
    // Given
    const ordinary = source.replace(/\.mdx$/, '.tsx')
    const config = {
      module: {
        rules: [
          {
            test: /\.tsx$/,
            issuer: /hidden/,
            enforce: 'pre' as const,
            use: [extraction],
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
      unknownFact: 'issuer',
    })
    expect('proof' in result).toBe(false)
  })

  it('returns no pipeline when native resource selection has no MDX segment', async () => {
    // Given / When
    const result = await withSelector(
      { module: { rules: [mdxRule()] } },
      async (selector) =>
        (await selector.selectPipeline(
          source.replace(/\.mdx$/, '.tsx'),
          new AbortController().signal,
        )) === undefined,
    )
    // Then
    expect(result).toBe(true)
  })

  it('blocks opaque use callbacks without executing them with invented origin facts', async () => {
    // Given
    let calls = 0
    const config: Configuration = {
      module: {
        rules: [
          {
            test: /\.mdx$/,
            use: function loaderFactory() {
              calls += 1
              return []
            },
          },
          mdxRule(),
        ],
      },
    }
    // When / Then
    await expect(
      withSelector(config, (selector) =>
        selector.selectPipeline(source, new AbortController().signal),
      ),
    ).rejects.toMatchObject({ unknownFact: 'use(context)' })
    expect(calls).toBe(0)
  })

  it('blocks opaque resource predicates without treating them as resource-only proof', async () => {
    // Given
    const calls: string[] = []
    const config = {
      module: {
        rules: [
          {
            ...mdxRule(),
            test(resource: string) {
              calls.push(resource)
              return true
            },
          },
        ],
      },
    }
    // When / Then
    await expect(
      withSelector(config, (selector) =>
        selector.selectPipeline(source, new AbortController().signal),
      ),
    ).rejects.toMatchObject({ unknownFact: 'test' })
    expect(calls).toEqual([''])
  })

  it('rejects downstream pitches even though their normal transforms run after Devup', async () => {
    // Given
    const pitch = fileURLToPath(
      new URL('./webpack-resource-pitch.cjs', import.meta.url),
    )
    const config = {
      module: { rules: [{ test: /\.mdx$/, use: [pitch] }, mdxRule()] },
    }
    // When / Then
    await expect(
      withSelector(config, (selector) =>
        selector.selectPipeline(source, new AbortController().signal),
      ),
    ).rejects.toMatchObject({ unknownFact: 'pitch/source substitution' })
  })

  it('retains original option comparison shells before later tuple mutation', async () => {
    // Given
    const options = { remarkPlugins: [['original-package', { visits: 0 }]] }
    // When
    const result = await withSelector(
      { module: { rules: [mdxRule(options)] } },
      async (selector) => {
        const selection = await selector.selectPipeline(
          source,
          new AbortController().signal,
        )
        if (!selection) throw new TypeError('missing selection')
        const before = snapshotMdxBindingValue(selection.loaders)
        options.remarkPlugins[0]?.splice(0, 1, 'wrapper-artifact')
        return {
          original: compareMdxBindingValues(selection.identity, before),
          changed: compareMdxBindingValues(
            selection.identity,
            snapshotMdxBindingValue(selection.loaders),
          ),
        }
      },
    )
    // Then
    expect(result.original).toBeUndefined()
    expect(result.changed?.path).toBe('loaders[0].options.remarkPlugins[0][0]')
  })

  it('preserves each selected original option value when resources share a loader module', async () => {
    // Given
    const first = { jsx: true }
    const second = { jsx: false }
    const config = {
      module: {
        rules: [
          { ...mdxRule(first), test: /first\.mdx$/ },
          { ...mdxRule(second), test: /second\.mdx$/ },
        ],
      },
    }
    // When
    const result = await withSelector(config, async (selector) => {
      const a = await selector.selectPipeline(
        source.replace('page.mdx', 'first.mdx'),
        new AbortController().signal,
      )
      const b = await selector.selectPipeline(
        source.replace('page.mdx', 'second.mdx'),
        new AbortController().signal,
      )
      return {
        a: a?.loaders[0]?.options,
        b: b?.loaders[0]?.options,
        watches: selector.watchInputs(),
      }
    })
    // Then
    expect(result.a).toBe(first)
    expect(result.b).toBe(second)
    expect(result.watches.length).toBeGreaterThan(0)
  })

  it('uses native RuleSet selection when oneOf resource branches compete', async () => {
    // Given
    const config = {
      module: {
        rules: [
          {
            oneOf: [
              { ...mdxRule({ jsx: false }), test: /never\.mdx$/ },
              mdxRule({ jsx: true }),
            ],
          },
        ],
      },
    }
    // When
    const result = await withSelector(
      config,
      async (selector, _compiler, binding) => {
        const selected = await selector.selectPipeline(
          source,
          new AbortController().signal,
        )
        const actual = nativeLoaders(
          nativeRuleSet(binding.params.normalModuleFactory).exec(
            resourceFacts(binding, source),
          ),
          'use',
        )
        return { selected, actual }
      },
    )
    // Then
    expect(result.selected?.loaders[0]?.options).toEqual({ jsx: true })
    expect(result.actual.at(-1)?.options).toEqual({ jsx: true })
  })
})
