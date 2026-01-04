import * as fs from 'node:fs'

import { afterEach, beforeEach, describe, expect, it, spyOn } from 'bun:test'

import { getPackageName } from '../get-package-name'

describe('getPackageName', () => {
  let readFileSyncSpy: ReturnType<typeof spyOn>

  beforeEach(() => {
    readFileSyncSpy = spyOn(fs, 'readFileSync') as unknown as ReturnType<
      typeof spyOn
    >
  })

  afterEach(() => {
    readFileSyncSpy.mockRestore()
  })

  it('should return the package name from package.json', () => {
    readFileSyncSpy.mockReturnValue(JSON.stringify({ name: 'my-package' }))
    const result = getPackageName('/path/to/package.json')
    expect(result).toBe('my-package')
    expect(readFileSyncSpy).toHaveBeenCalledWith(
      '/path/to/package.json',
      'utf-8',
    )
  })

  it('should return scoped package name', () => {
    readFileSyncSpy.mockReturnValue(
      JSON.stringify({ name: '@scope/my-package' }),
    )
    const result = getPackageName('/path/to/package.json')
    expect(result).toBe('@scope/my-package')
  })

  it('should return undefined if name is not present', () => {
    readFileSyncSpy.mockReturnValue(JSON.stringify({ version: '1.0.0' }))
    const result = getPackageName('/path/to/package.json')
    expect(result).toBeUndefined()
  })
})
