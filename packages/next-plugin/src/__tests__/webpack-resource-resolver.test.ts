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
      aliases: [{ name: 'provider', onlyModule: true, alias: false }],
      resolution: { ignored: true },
    })
  })

  it.each([{ descriptor: true }, { descriptor: false }])(
    'retains injected shared mixed candidates without claiming native Factory acceptance %j',
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
        aliases: descriptor
          ? [{ name: 'provider', onlyModule: true, alias: candidates }]
          : { provider$: candidates },
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
    expect(result).toEqual([
      { name: 'provider', onlyModule: true, alias: ['first', 'second'] },
    ])
  })

  it('selects a reached false before a later duplicate descriptor', async () => {
    // Given
    const config: Configuration = {
      resolve: {
        alias: [
          { name: 'provider', alias: false },
          { name: 'provider', alias: 'second' },
        ],
      },
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
    expect(aliases).toEqual([
      { name: 'constructor', alias: 'first' },
      { name: '__proto__', alias: 'second' },
    ])
  })

  it('detaches frozen descriptor shells and candidates without freezing caller state', async () => {
    // Given
    const candidates = [mdxLoader]
    const descriptor = { name: 'provider', alias: candidates, onlyModule: true }
    const alias = [descriptor]
    // When
    const result = await withSelector(
      { resolve: { alias }, module: { rules: [mdxRule()] } },
      (selector, compiler) => {
        candidates.splice(0, 1, 'missing')
        descriptor.name = 'changed'
        descriptor.onlyModule = false
        alias.push({ name: 'provider', alias: ['later'], onlyModule: false })
        return {
          resolution: createModuleResolver({
            cwd: compiler.context,
            alias: selector.aliases,
          })('provider', 'page.tsx'),
          frozen:
            Object.isFrozen(selector.aliases) &&
            Object.values(selector.aliases).every(
              (entry) => Object.isFrozen(entry) && Object.isFrozen(entry.alias),
            ),
        }
      },
    )
    // Then
    expect(result.resolution?.path).toBe(mdxLoader)
    expect(result.frozen).toBe(true)
    expect(candidates).toEqual(['missing'])
    expect(descriptor.name).toBe('changed')
  })
})
