import { describe, expect, it } from 'bun:test'
import type { Configuration } from 'webpack'

import { mdxRule, withSelector } from './webpack-resource-fixture'

describe('native resolver representation', () => {
  it('refuses disabled aliases instead of silently changing native resolver meaning', async () => {
    // Given
    const config: Configuration = {
      resolve: { alias: { provider: false } },
      module: { rules: [mdxRule()] },
    }
    // When / Then
    await expect(withSelector(config, () => true)).rejects.toBeInstanceOf(
      TypeError,
    )
  })

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
