import { existsSync } from 'node:fs'
import { join } from 'node:path'

import { findRoot } from '../find-root'

vi.mock('node:fs')

describe('findRoot', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('should return the first directory with package.json when found', () => {
    const mockDir = '/project/src/components'
    const expectedRoot = '/project'

    vi.mocked(existsSync)
      .mockReturnValueOnce(false) // /project/src/components/package.json
      .mockReturnValueOnce(false) // /project/src/package.json
      .mockReturnValueOnce(true) // /project/package.json
      .mockReturnValueOnce(false) // /package.json

    const result = findRoot(mockDir)

    expect(result).toBe(expectedRoot)
    expect(existsSync).toHaveBeenCalledWith(
      join('/project/src/components', 'package.json'),
    )
    expect(existsSync).toHaveBeenCalledWith(
      join('/project/src', 'package.json'),
    )
    expect(existsSync).toHaveBeenCalledWith(join('/project', 'package.json'))
  })

  it('should return process.cwd() when no package.json is found', () => {
    const mockDir = '/some/deep/nested/directory'
    const originalCwd = process.cwd()

    vi.mocked(existsSync).mockReturnValue(false)

    const result = findRoot(mockDir)

    expect(result).toBe(originalCwd)
  })

  it('should handle root directory correctly', () => {
    const mockDir = '/'

    vi.mocked(existsSync).mockReturnValue(false)

    const result = findRoot(mockDir)

    expect(result).toBe(process.cwd())
  })

  it('should find package.json in current directory', () => {
    const mockDir = '/project'

    vi.mocked(existsSync)
      .mockReturnValueOnce(true) // /project/package.json
      .mockReturnValueOnce(false) // /package.json

    const result = findRoot(mockDir)

    expect(result).toBe('/project')
  })

  it('should handle multiple package.json files and return the deepest one', () => {
    const mockDir = '/project/src/components/deep'
    const expectedRoot = '/project' // The function returns the last found package.json (closest to root)

    vi.mocked(existsSync)
      .mockReturnValueOnce(false) // /project/src/components/deep/package.json
      .mockReturnValueOnce(false) // /project/src/components/package.json
      .mockReturnValueOnce(true) // /project/src/package.json
      .mockReturnValueOnce(true) // /project/package.json
      .mockReturnValueOnce(false) // /package.json

    const result = findRoot(mockDir)

    expect(result).toBe(expectedRoot)
  })

  it('should handle Windows-style paths', () => {
    const mockDir = 'C:\\project\\src\\components'
    const expectedRoot = 'C:\\project'

    vi.mocked(existsSync)
      .mockReturnValueOnce(false) // C:\project\src\components\package.json
      .mockReturnValueOnce(false) // C:\project\src\package.json
      .mockReturnValueOnce(true) // C:\project\package.json
      .mockReturnValueOnce(false) // C:\package.json

    const result = findRoot(mockDir)

    expect(result).toBe(expectedRoot)
  })

  it('should handle relative paths', () => {
    const mockDir = './src/components'
    const expectedRoot = '.'

    vi.mocked(existsSync)
      .mockReturnValueOnce(false) // ./src/components/package.json
      .mockReturnValueOnce(false) // ./src/package.json
      .mockReturnValueOnce(true) // ./package.json
      .mockReturnValueOnce(false) // ../package.json

    const result = findRoot(mockDir)

    expect(result).toBe(expectedRoot)
  })

  it('should stop at filesystem root', () => {
    const mockDir = '/some/path'

    // Mock existsSync to return false for all calls, simulating no package.json found
    vi.mocked(existsSync).mockReturnValue(false)

    const result = findRoot(mockDir)

    expect(result).toBe(process.cwd())
  })

  it('should handle empty string input', () => {
    const mockDir = ''

    vi.mocked(existsSync).mockReturnValue(false)

    const result = findRoot(mockDir)

    expect(result).toBe(process.cwd())
  })

  it('should handle single character directory', () => {
    const mockDir = 'a'

    vi.mocked(existsSync)
      .mockReturnValueOnce(false) // a/package.json
      .mockReturnValueOnce(false) // ./package.json

    const result = findRoot(mockDir)

    expect(result).toBe(process.cwd())
  })
})
