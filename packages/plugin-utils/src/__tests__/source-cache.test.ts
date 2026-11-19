import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'bun:test'

import {
  fileStamp,
  readSource,
  type SourceCache,
  withSourceCache,
} from '../source-cache'

describe('readSource', () => {
  let dir: string
  let file: string

  beforeEach(() => {
    dir = mkdtempSync(join(tmpdir(), 'devup-ui-source-cache-'))
    file = join(dir, 'a.tsx')
    writeFileSync(file, 'on disk')
  })

  afterEach(() => {
    rmSync(dir, { recursive: true, force: true })
  })

  /** A cache already holding `code` for the file as it is now */
  function holding(code: string): SourceCache {
    return new Map([[resolve(file), { stamp: fileStamp(file) ?? '', code }]])
  }

  it('reads the disk outside a cache', () => {
    expect(readSource(file)).toBe('on disk')
    expect(globalThis.__devupSourceCache).toBeUndefined()
  })

  it('serves an unchanged file from the cache, under any spelling of its path', () => {
    // Given
    const cache = holding('cached')
    // When
    const read = withSourceCache(
      () => [readSource(file), readSource(join(dir, '.', 'a.tsx'))],
      cache,
    )
    // Then
    expect(read).toEqual(['cached', 'cached'])
  })

  it('reads a changed file again and keeps it', () => {
    // Given
    const cache = holding('stale')
    // When
    writeFileSync(file, 'changed text')
    // Then
    expect(withSourceCache(() => readSource(file), cache)).toBe('changed text')
    expect(cache.get(resolve(file))).toEqual({
      stamp: fileStamp(file) ?? '',
      code: 'changed text',
    })
  })

  it('reads a file once into an empty cache', () => {
    const cache: SourceCache = new Map()
    expect(withSourceCache(() => readSource(file), cache)).toBe('on disk')
    expect(cache.get(resolve(file))?.code).toBe('on disk')
  })

  it('reports a deleted file and forgets it', () => {
    // Given
    const cache = holding('cached')
    // When
    rmSync(file)
    // Then
    expect(() => withSourceCache(() => readSource(file), cache)).toThrow()
    expect(cache.size).toBe(0)
  })

  it('keeps the outer cache in a nested run and restores none after', () => {
    // Given
    const outer: SourceCache = new Map()
    const inner: SourceCache = new Map()
    // When
    withSourceCache(() => withSourceCache(() => readSource(file), inner), outer)
    // Then
    expect(outer.size).toBe(1)
    expect(inner.size).toBe(0)
    expect(globalThis.__devupSourceCache).toBeUndefined()
  })

  it('restores the outer cache when the run throws', () => {
    expect(() =>
      withSourceCache(() => {
        throw new Error('boom')
      }, new Map()),
    ).toThrow('boom')
    expect(globalThis.__devupSourceCache).toBeUndefined()
  })

  it('stamps no missing file', () => {
    expect(fileStamp(join(dir, 'missing.tsx'))).toBeUndefined()
  })
})
