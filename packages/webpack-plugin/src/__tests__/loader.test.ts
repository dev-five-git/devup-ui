import { writeFile } from 'node:fs/promises'
import { join } from 'node:path'

import {
  codeExtract,
  exportClassMap,
  exportFileMap,
  exportSheet,
} from '@devup-ui/wasm'

import devupUILoader from '../loader'

vi.mock('@devup-ui/wasm')
vi.mock('node:fs/promises')

beforeEach(() => {
  vi.resetAllMocks()
  Date.now = vi.fn().mockReturnValue(0)
})

describe('devupUILoader', () => {
  it('should extract code with css', async () => {
    const _compiler = {
      __DEVUP_CACHE: '',
    }
    const t = {
      getOptions: () => ({
        package: 'package',
        cssDir: 'cssFile',
        sheetFile: 'sheetFile',
        classMapFile: 'classMapFile',
        fileMapFile: 'fileMapFile',
        watch: true,
        singleCss: true,
      }),
      async: vi.fn().mockReturnValue(vi.fn()),
      resourcePath: 'index.tsx',
      addDependency: vi.fn(),
      _compiler,
    }
    vi.mocked(exportSheet).mockReturnValue('sheet')
    vi.mocked(exportClassMap).mockReturnValue('classMap')
    vi.mocked(exportFileMap).mockReturnValue('fileMap')

    vi.mocked(codeExtract).mockReturnValue({
      code: 'code',
      css: 'css',
      free: vi.fn(),
      map: '{}',
      css_file: 'cssFile',
    })
    devupUILoader.bind(t as any)(Buffer.from('code'), 'index.tsx')

    expect(t.async).toHaveBeenCalled()
    expect(codeExtract).toHaveBeenCalledWith(
      'index.tsx',
      'code',
      'package',
      './cssFile',
      true,
    )
    await vi.waitFor(() => {
      expect(t.async()).toHaveBeenCalledWith(null, 'code', {})
      expect(writeFile).toHaveBeenCalledWith(
        join('cssFile', 'cssFile'),
        '/* index.tsx 0 */',
      )
      expect(writeFile).toHaveBeenCalledWith('sheetFile', 'sheet')
      expect(writeFile).toHaveBeenCalledWith('classMapFile', 'classMap')
      expect(writeFile).toHaveBeenCalledWith('fileMapFile', 'fileMap')
    })

    expect(t._compiler.__DEVUP_CACHE).toBe('index.tsx 0')
  })

  it('should extract code without css', () => {
    const t = {
      getOptions: () => ({
        package: 'package',
        cssDir: 'cssFile',
        watch: false,
        singleCss: true,
      }),
      async: vi.fn().mockReturnValue(vi.fn()),
      resourcePath: 'index.tsx',
      addDependency: vi.fn(),
    }
    vi.mocked(codeExtract).mockReturnValue({
      code: 'code',
      css: undefined,
      free: vi.fn(),
      map: undefined,
      css_file: 'cssFile',
    })
    devupUILoader.bind(t as any)(Buffer.from('code'), 'index.tsx')

    expect(t.async).toHaveBeenCalled()
    expect(codeExtract).toHaveBeenCalledWith(
      'index.tsx',
      'code',
      'package',
      './cssFile',
      true,
    )
    expect(t.async()).toHaveBeenCalledWith(null, 'code', null)
    expect(writeFile).not.toHaveBeenCalledWith('cssFile', 'css', {
      encoding: 'utf-8',
    })
  })

  it('should handle error', () => {
    const t = {
      getOptions: () => ({
        package: 'package',
        cssDir: 'cssFile',
        watch: false,
        singleCss: true,
      }),
      async: vi.fn().mockReturnValue(vi.fn()),
      resourcePath: 'index.tsx',
      addDependency: vi.fn(),
    }
    vi.mocked(codeExtract).mockImplementation(() => {
      throw new Error('error')
    })
    devupUILoader.bind(t as any)(Buffer.from('code'), 'index.tsx')

    expect(t.async).toHaveBeenCalled()
    expect(t.async()).toHaveBeenCalledWith(new Error('error'))
  })

  it('should load with date now on watch', () => {
    const t = {
      getOptions: () => ({
        package: 'package',
        cssDir: 'cssFile',
        watch: true,
        singleCss: true,
      }),
      async: vi.fn().mockReturnValue(vi.fn()),
      resourcePath: 'index.tsx',
      addDependency: vi.fn(),
    }
    vi.mocked(codeExtract).mockReturnValue({
      code: 'code',
      css: 'css',
      free: vi.fn(),
      map: undefined,
      css_file: 'cssFile',
    })
    devupUILoader.bind(t as any)(Buffer.from('code'), 'index.tsx')

    expect(t.async).toHaveBeenCalled()
    expect(codeExtract).toHaveBeenCalledWith(
      'index.tsx',
      'code',
      'package',
      './cssFile',
      true,
    )
  })

  it('should load with nowatch', () => {
    const t = {
      getOptions: () => ({
        package: 'package',
        cssDir: 'cssFile',
        watch: false,
        singleCss: true,
      }),
      async: vi.fn().mockReturnValue(vi.fn()),
      resourcePath: 'index.tsx',
      addDependency: vi.fn(),
    }
    vi.mocked(codeExtract).mockReturnValue({
      code: 'code',
      css: 'css',
      free: vi.fn(),
      map: undefined,
      css_file: 'cssFile',
    })
    devupUILoader.bind(t as any)(Buffer.from('code'), 'index.tsx')
  })
})
