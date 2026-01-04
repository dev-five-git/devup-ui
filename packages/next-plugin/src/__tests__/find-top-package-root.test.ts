import * as fs from 'node:fs'
import { join } from 'node:path'

import { afterEach, beforeEach, describe, expect, it, spyOn } from 'bun:test'

import { findTopPackageRoot } from '../find-top-package-root'

describe('findTopPackageRoot', () => {
  let existsSyncSpy: ReturnType<typeof spyOn>

  beforeEach(() => {
    existsSyncSpy = spyOn(fs, 'existsSync') as unknown as ReturnType<
      typeof spyOn
    >
  })

  afterEach(() => {
    existsSyncSpy.mockRestore()
  })

  it('should return pwd when no package.json is found', () => {
    existsSyncSpy.mockReturnValue(false)
    const result = findTopPackageRoot('/some/path')
    expect(result).toBe('/some/path')
  })

  it('should return the top-most directory with package.json', () => {
    existsSyncSpy.mockImplementation((path: string) => {
      return (
        path === join('/root/project', 'package.json') ||
        path === join('/root', 'package.json')
      )
    })
    const result = findTopPackageRoot('/root/project/nested')
    expect(result).toBe('/root')
  })

  it('should return the current directory if it has package.json and is root', () => {
    existsSyncSpy.mockImplementation((path: string) => {
      return path === join('/', 'package.json')
    })
    const result = findTopPackageRoot('/')
    expect(result).toBe('/')
  })

  it('should use process.cwd() as default pwd', () => {
    const originalCwd = process.cwd()
    existsSyncSpy.mockImplementation((path: string) => {
      return path === join(originalCwd, 'package.json')
    })
    const result = findTopPackageRoot()
    expect(result).toBe(originalCwd)
  })

  it('should traverse up and find the topmost package.json', () => {
    existsSyncSpy.mockImplementation((path: string) => {
      return path === join('/a/b/c', 'package.json')
    })
    const result = findTopPackageRoot('/a/b/c/d/e')
    expect(result).toBe('/a/b/c')
  })
})
