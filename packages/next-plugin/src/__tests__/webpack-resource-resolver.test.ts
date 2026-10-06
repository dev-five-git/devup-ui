import { createModuleResolver } from '@devup-ui/plugin-utils'
import { describe, expect, it } from 'bun:test'
import type { Configuration } from 'webpack'

import { webpackResourceResolver } from '../webpack-resource-delivery'
import { mdxLoader, mdxRule, withSelector } from './webpack-resource-fixture'

describe('native resolver representation', () => {
  it('resolves disabled aliases as ignored without manufacturing a module', async () => {
    // Given
    const config: Configuration = {
      resolve: { alias: { provider: false } },
      module: { rules: [mdxRule()] },
    }
    // When
    const result = await withSelector(config, (selector, compiler) =>
      createModuleResolver({ cwd: compiler.context, alias: selector.aliases })(
        'provider',
        'page.tsx',
      ),
    )
    // Then
    expect(result).toEqual({ ignored: true })
  })

  it('preserves an exact ignored descriptor as false', async () => {
    // Given
    const config: Configuration = {
      resolve: {
        alias: [{ name: 'provider', onlyModule: true, alias: false }],
      },
      module: { rules: [mdxRule()] },
    }
    // When
    const result = await withSelector(config, (selector, compiler) => ({
      aliases: selector.aliases,
      resolution: createModuleResolver({
        cwd: compiler.context,
        alias: selector.aliases,
      })('provider', 'page.tsx'),
    }))
    // Then
    expect(result).toEqual({
      aliases: { provider$: false },
      resolution: { ignored: true },
    })
  })

  it.each([{ descriptor: true }, { descriptor: false }])(
    'retains ordered false candidates at the effective resolver boundary %j',
    async ({ descriptor }) => {
      // Given
      const candidates = ['missing-provider', false, mdxLoader] as const
      const config: Configuration = {
        module: { rules: [mdxRule()] },
      }
      // When
      const result = await withSelector(config, (_, compiler, binding) => {
        const effectiveConfiguration = {
          ...binding.effectiveConfiguration,
          resolve: Object.defineProperty(
            { ...binding.effectiveConfiguration.resolve },
            'alias',
            {
              value: descriptor
                ? [{ name: 'provider', onlyModule: true, alias: candidates }]
                : { provider$: candidates },
              enumerable: true,
            },
          ),
        }
        const settings = webpackResourceResolver({
          ...binding,
          effectiveConfiguration,
        })
        return {
          aliases: settings.aliases,
          resolution: createModuleResolver({
            cwd: compiler.context,
            alias: settings.aliases,
          })('provider', 'page.tsx'),
        }
      })
      // Then
      expect(result).toEqual({
        aliases: { provider$: candidates },
        resolution: { ignored: true },
      })
    },
  )

  it('preserves unique alias descriptors and candidate order', async () => {
    // Given
    const config: Configuration = {
      resolve: {
        alias: [
          { name: 'provider', onlyModule: true, alias: ['first', 'second'] },
        ],
      },
      module: { rules: [mdxRule()] },
    }
    // When
    const result = await withSelector(config, (selector) => selector.aliases)
    // Then
    expect(result).toEqual({ provider$: ['first', 'second'] })
  })

  it('refuses duplicate alias descriptors rather than selecting the last descriptor', async () => {
    // Given
    const config: Configuration = {
      resolve: {
        alias: [
          { name: 'provider', alias: 'first' },
          { name: 'provider', alias: 'second' },
        ],
      },
      module: { rules: [mdxRule()] },
    }
    // When / Then
    await expect(withSelector(config, () => true)).rejects.toBeInstanceOf(
      TypeError,
    )
  })

  it('preserves string aliases without converting them into unordered candidates', async () => {
    // Given
    const config: Configuration = {
      resolve: { alias: { provider: 'first' } },
      module: { rules: [mdxRule()] },
    }
    // When
    const result = await withSelector(config, (selector) => selector.aliases)
    // Then
    expect(result).toEqual({ provider: 'first' })
  })
  it('preserves unique aliases named like Object prototype properties', async () => {
    // Given
    const config: Configuration = {
      resolve: {
        alias: [
          { name: 'constructor', alias: 'first' },
          { name: '__proto__', alias: 'second' },
        ],
      },
      module: { rules: [mdxRule()] },
    }
    // When
    const aliases = await withSelector(config, (selector) => selector.aliases)
    // Then
    expect(Object.hasOwn(aliases, 'constructor')).toBe(true)
    expect(Object.hasOwn(aliases, '__proto__')).toBe(true)
    expect(aliases['__proto__']).toEqual(['second'])
    expect(Object.getPrototypeOf(aliases)).toBe(Object.prototype)
  })
})
