import type { PathLike } from 'node:fs'
import { join } from 'node:path'

import { describe, expect, it, vi } from 'vitest'

import { findTopPackageRoot } from '../find-top-package-root'
import { getPackageName } from '../get-package-name'
import { hasLocalPackage } from '../has-localpackage'

vi.mock('node:fs', () => ({
  existsSync: vi.fn(),
  readFileSync: vi.fn(),
}))

const { existsSync, readFileSync } = await import('node:fs')

describe('findTopPackageRoot', () => {
  it('returns highest directory containing package.json', () => {
    const root = join('/', 'repo')
    const child = join(root, 'packages', 'pkg')
    vi.mocked(existsSync).mockImplementation((path: PathLike) => {
      if (path === join(root, 'package.json')) return true
      return false
    })

    const result = findTopPackageRoot(child)

    expect(result).toBe(root)
  })

  it('falls back to cwd when no package.json found', () => {
    const cwd = join('/', 'repo', 'packages', 'pkg')
    vi.mocked(existsSync).mockReturnValue(false)

    const result = findTopPackageRoot(cwd)

    expect(result).toBe(cwd)
  })
})

describe('hasLocalPackage', () => {
  it('detects workspace dependency', () => {
    vi.mocked(readFileSync).mockReturnValue(
      JSON.stringify({
        dependencies: {
          foo: 'workspace:*',
          bar: '^1.0.0',
        },
      }),
    )

    expect(hasLocalPackage()).toBe(true)
  })

  it('returns false when no workspace dependency', () => {
    vi.mocked(readFileSync).mockReturnValue(
      JSON.stringify({
        dependencies: {
          foo: '^1.0.0',
        },
      }),
    )

    expect(hasLocalPackage()).toBe(false)
  })

  it('returns false when dependencies field is missing', () => {
    vi.mocked(readFileSync).mockReturnValue('{}')

    expect(hasLocalPackage()).toBe(false)
  })
})

describe('getPackageName', () => {
  it('reads and returns package name', () => {
    vi.mocked(readFileSync).mockReturnValue(
      JSON.stringify({ name: '@scope/pkg' }),
    )

    expect(getPackageName('/path/package.json')).toBe('@scope/pkg')
    expect(readFileSync).toHaveBeenCalledWith('/path/package.json', 'utf-8')
  })
})
