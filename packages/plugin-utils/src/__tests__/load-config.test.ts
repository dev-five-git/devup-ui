import { mkdirSync, rmSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { afterAll, beforeAll, describe, expect, it } from 'bun:test'

import { deepMerge, loadDevupConfig, loadDevupConfigSync } from '../load-config'

const testDir = join(import.meta.dir, 'fixtures')

beforeAll(() => {
  mkdirSync(testDir, { recursive: true })
})

afterAll(() => {
  rmSync(testDir, { recursive: true, force: true })
})

describe('deepMerge', () => {
  it('should merge two flat objects', () => {
    const base = { a: 1, b: 2 }
    const override = { b: 3, c: 4 }
    expect(deepMerge(base, override) as any).toEqual({ a: 1, b: 3, c: 4 })
  })

  it('should deep merge nested objects', () => {
    const base = {
      theme: {
        colors: { default: { text: 'black' } },
      },
    }
    const override = {
      theme: {
        colors: { default: { bg: 'white' } },
      },
    }
    expect(deepMerge(base, override)).toEqual({
      theme: {
        colors: { default: { text: 'black', bg: 'white' } },
      },
    })
  })

  it('should replace arrays', () => {
    const base = { arr: [1, 2, 3] }
    const override = { arr: [4, 5] }
    expect(deepMerge(base, override)).toEqual({ arr: [4, 5] })
  })

  it('should handle undefined values in override', () => {
    const base = { a: 1, b: 2 }
    const override = { a: undefined, c: 3 }
    expect(deepMerge(base, override)).toEqual({ a: 1, b: 2, c: 3 })
  })

  it('should return override when base is not a plain object', () => {
    expect(deepMerge([1, 2], { a: 1 })).toEqual({ a: 1 })
    expect(deepMerge(null, { a: 1 })).toEqual({ a: 1 })
    expect(deepMerge('string', { a: 1 })).toEqual({ a: 1 })
    expect(deepMerge(123, { a: 1 })).toEqual({ a: 1 })
  })

  it('should return override when override is not a plain object', () => {
    expect(deepMerge({ a: 1 }, [1, 2])).toEqual([1, 2])
    expect(deepMerge({ a: 1 }, null)).toEqual(null)
    expect(deepMerge({ a: 1 }, 'string')).toEqual('string')
  })

  it('should return base when override is undefined', () => {
    expect(deepMerge({ a: 1 }, undefined)).toEqual({ a: 1 })
  })
})

describe('loadDevupConfigSync', () => {
  it('should return empty object for non-existent file', () => {
    const result = loadDevupConfigSync(join(testDir, 'non-existent.json'))
    expect(result).toEqual({})
  })

  it('should load simple config', () => {
    const configPath = join(testDir, 'simple.json')
    writeFileSync(
      configPath,
      JSON.stringify({
        theme: { colors: { default: { text: 'red' } } },
      }),
    )

    const result = loadDevupConfigSync(configPath)
    expect(result).toEqual({
      theme: { colors: { default: { text: 'red' } } },
    })
  })

  it('should return empty object for invalid JSON', () => {
    const configPath = join(testDir, 'invalid.json')
    writeFileSync(configPath, 'not valid json {{{')

    const result = loadDevupConfigSync(configPath)
    expect(result).toEqual({})
  })

  it('should resolve extends', () => {
    const baseDir = join(testDir, 'extends')
    mkdirSync(baseDir, { recursive: true })

    // Base config
    writeFileSync(
      join(baseDir, 'base.json'),
      JSON.stringify({
        theme: {
          colors: { default: { text: 'black', bg: 'white' } },
        },
      }),
    )

    // Child config that extends base
    writeFileSync(
      join(baseDir, 'child.json'),
      JSON.stringify({
        extends: ['./base.json'],
        theme: {
          colors: { default: { text: 'red' } },
        },
      }),
    )

    const result = loadDevupConfigSync(join(baseDir, 'child.json'))
    expect(result).toEqual({
      theme: {
        colors: { default: { text: 'red', bg: 'white' } },
      },
    })
  })

  it('should resolve multiple extends in order', () => {
    const baseDir = join(testDir, 'multi-extends')
    mkdirSync(baseDir, { recursive: true })

    // First base
    writeFileSync(
      join(baseDir, 'first.json'),
      JSON.stringify({
        theme: { colors: { default: { a: '1', b: '2' } } },
      }),
    )

    // Second base (overrides first)
    writeFileSync(
      join(baseDir, 'second.json'),
      JSON.stringify({
        theme: { colors: { default: { b: '3', c: '4' } } },
      }),
    )

    // Child extends both
    writeFileSync(
      join(baseDir, 'child.json'),
      JSON.stringify({
        extends: ['./first.json', './second.json'],
        theme: { colors: { default: { c: '5' } } },
      }),
    )

    const result = loadDevupConfigSync(join(baseDir, 'child.json'))
    expect(result).toEqual({
      theme: {
        colors: { default: { a: '1', b: '3', c: '5' } },
      },
    })
  })

  it('should handle nested extends', () => {
    const baseDir = join(testDir, 'nested-extends')
    mkdirSync(baseDir, { recursive: true })

    // Grandparent
    writeFileSync(
      join(baseDir, 'grandparent.json'),
      JSON.stringify({
        theme: { colors: { default: { x: '1' } } },
      }),
    )

    // Parent extends grandparent
    writeFileSync(
      join(baseDir, 'parent.json'),
      JSON.stringify({
        extends: ['./grandparent.json'],
        theme: { colors: { default: { y: '2' } } },
      }),
    )

    // Child extends parent
    writeFileSync(
      join(baseDir, 'child.json'),
      JSON.stringify({
        extends: ['./parent.json'],
        theme: { colors: { default: { z: '3' } } },
      }),
    )

    const result = loadDevupConfigSync(join(baseDir, 'child.json'))
    expect(result).toEqual({
      theme: {
        colors: { default: { x: '1', y: '2', z: '3' } },
      },
    })
  })
})

describe('loadDevupConfig', () => {
  it('should return empty object for non-existent file', async () => {
    const result = await loadDevupConfig(join(testDir, 'non-existent.json'))
    expect(result).toEqual({})
  })

  it('should load and resolve extends asynchronously', async () => {
    const baseDir = join(testDir, 'async-extends')
    mkdirSync(baseDir, { recursive: true })

    writeFileSync(
      join(baseDir, 'base.json'),
      JSON.stringify({
        theme: { colors: { default: { text: 'blue' } } },
      }),
    )

    writeFileSync(
      join(baseDir, 'child.json'),
      JSON.stringify({
        extends: ['./base.json'],
        theme: { colors: { dark: { text: 'white' } } },
      }),
    )

    const result = await loadDevupConfig(join(baseDir, 'child.json'))
    expect(result).toEqual({
      theme: {
        colors: {
          default: { text: 'blue' },
          dark: { text: 'white' },
        },
      },
    })
  })
})
