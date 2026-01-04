import * as fs from 'node:fs'

import { afterEach, beforeEach, describe, expect, it, spyOn } from 'bun:test'

import { hasLocalPackage } from '../has-localpackage'

describe('hasLocalPackage', () => {
  let readFileSyncSpy: ReturnType<typeof spyOn>

  beforeEach(() => {
    readFileSyncSpy = spyOn(fs, 'readFileSync') as unknown as ReturnType<
      typeof spyOn
    >
  })

  afterEach(() => {
    readFileSyncSpy.mockRestore()
  })

  it('should return true when dependencies contain workspace: protocol', () => {
    readFileSyncSpy.mockReturnValue(
      JSON.stringify({
        dependencies: {
          'local-pkg': 'workspace:*',
          'external-pkg': '^1.0.0',
        },
      }),
    )
    expect(hasLocalPackage()).toBe(true)
  })

  it('should return false when no workspace: dependencies', () => {
    readFileSyncSpy.mockReturnValue(
      JSON.stringify({
        dependencies: {
          'external-pkg': '^1.0.0',
          'another-pkg': '~2.0.0',
        },
      }),
    )
    expect(hasLocalPackage()).toBe(false)
  })

  it('should return false when dependencies is empty', () => {
    readFileSyncSpy.mockReturnValue(
      JSON.stringify({
        dependencies: {},
      }),
    )
    expect(hasLocalPackage()).toBe(false)
  })

  it('should return false when dependencies is undefined', () => {
    readFileSyncSpy.mockReturnValue(
      JSON.stringify({
        name: 'test-package',
      }),
    )
    expect(hasLocalPackage()).toBe(false)
  })

  it('should handle workspace:^ protocol', () => {
    readFileSyncSpy.mockReturnValue(
      JSON.stringify({
        dependencies: {
          'local-pkg': 'workspace:^',
        },
      }),
    )
    expect(hasLocalPackage()).toBe(true)
  })

  it('should handle workspace:~ protocol', () => {
    readFileSyncSpy.mockReturnValue(
      JSON.stringify({
        dependencies: {
          'local-pkg': 'workspace:~1.0.0',
        },
      }),
    )
    expect(hasLocalPackage()).toBe(true)
  })
})
