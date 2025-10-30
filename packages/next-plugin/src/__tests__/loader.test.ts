import { writeFile } from 'node:fs/promises'
import { join, relative } from 'node:path'

import {
  codeExtract,
  exportClassMap,
  exportFileMap,
  exportSheet,
  getCss,
  importClassMap,
  importFileMap,
  importSheet,
  registerTheme,
} from '@devup-ui/wasm'

vi.mock('@devup-ui/wasm')
vi.mock('node:fs/promises')
vi.mock('node:path', async (original: any) => {
  const origin = await original()
  return {
    ...origin,
    relative: vi.fn(origin.relative),
  }
})

beforeEach(() => {
  vi.resetAllMocks()
  vi.resetModules()
  Date.now = vi.fn().mockReturnValue(0)
})

describe('devupUILoader', () => {
  it.each(
    createTestMatrix({
      updatedBaseStyle: [true, false],
    }),
  )('should extract code with css', async (options) => {
    const { default: devupUILoader } = await import('../loader')
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
    vi.mocked(getCss).mockReturnValue('css')

    vi.mocked(codeExtract).mockReturnValue({
      code: 'code',
      css: 'css',
      free: vi.fn(),
      map: '{}',
      cssFile: 'cssFile',
      updatedBaseStyle: options.updatedBaseStyle,
      [Symbol.dispose]: vi.fn(),
    })
    devupUILoader.bind(t as any)(Buffer.from('code'), 'index.tsx')

    expect(t.async).toHaveBeenCalled()
    expect(codeExtract).toHaveBeenCalledWith(
      'index.tsx',
      'code',
      'package',
      './cssFile',
      true,
      false,
      true,
    )
    if (options.updatedBaseStyle) {
      expect(writeFile).toHaveBeenCalledWith(
        join('cssFile', 'devup-ui.css'),
        'css',
        'utf-8',
      )
    } else {
      expect(writeFile).not.toHaveBeenCalledWith(
        join('cssFile', 'devup-ui.css'),
        'css',
        'utf-8',
      )
    }
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
  })

  it('should extract code without css', async () => {
    const { default: devupUILoader } = await import('../loader')
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
      cssFile: undefined,
      updatedBaseStyle: false,
      [Symbol.dispose]: vi.fn(),
    })
    devupUILoader.bind(t as any)(Buffer.from('code'), 'index.tsx')

    expect(t.async).toHaveBeenCalled()
    expect(codeExtract).toHaveBeenCalledWith(
      'index.tsx',
      'code',
      'package',
      './cssFile',
      true,
      false,
      true,
    )
    await vi.waitFor(() => {
      expect(t.async()).toHaveBeenCalledWith(null, 'code', null)
    })
    expect(writeFile).not.toHaveBeenCalledWith('cssFile', 'css', {
      encoding: 'utf-8',
    })
  })

  it('should handle error', async () => {
    const { default: devupUILoader } = await import('../loader')
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

  it('should load with date now on watch', async () => {
    const { default: devupUILoader } = await import('../loader')
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
      cssFile: 'cssFile',
      updatedBaseStyle: false,
      [Symbol.dispose]: vi.fn(),
    })
    devupUILoader.bind(t as any)(Buffer.from('code'), 'index.tsx')

    expect(t.async).toHaveBeenCalled()
    expect(codeExtract).toHaveBeenCalledWith(
      'index.tsx',
      'code',
      'package',
      './cssFile',
      true,
      false,
      true,
    )
  })

  it('should load with nowatch', async () => {
    const { default: devupUILoader } = await import('../loader')
    const t = {
      getOptions: () => ({
        package: 'package',
        cssDir: './foo',
        watch: false,
        singleCss: true,
      }),
      async: vi.fn().mockReturnValue(vi.fn()),
      resourcePath: './foo/index.tsx',
      addDependency: vi.fn(),
    }
    vi.mocked(codeExtract).mockReturnValue({
      code: 'code',
      css: 'css',
      free: vi.fn(),
      map: undefined,
      cssFile: 'cssFile',
      updatedBaseStyle: false,
      [Symbol.dispose]: vi.fn(),
    })
    vi.mocked(relative).mockReturnValue('./foo/index.tsx')
    devupUILoader.bind(t as any)(Buffer.from('code'), '/foo/index.tsx')
  })
  it('should load with theme', async () => {
    const { default: devupUILoader } = await import('../loader')
    const t = {
      getOptions: () => ({
        package: 'package',
        cssDir: 'cssFile',
        watch: false,
        singleCss: true,
        theme: {
          colors: {
            primary: '#000',
          },
        },
      }),
      async: vi.fn().mockReturnValue(vi.fn()),
      resourcePath: 'index.tsx',
      addDependency: vi.fn(),
    }
    vi.mocked(registerTheme).mockReturnValueOnce(undefined)
    vi.mocked(codeExtract).mockReturnValue({
      code: 'code',
      css: 'css',
      free: vi.fn(),
      map: undefined,
      cssFile: 'cssFile',
      updatedBaseStyle: false,
      [Symbol.dispose]: vi.fn(),
    })
    devupUILoader.bind(t as any)(Buffer.from('code'), 'index.tsx')
    expect(registerTheme).toHaveBeenCalledWith({
      colors: {
        primary: '#000',
      },
    })
  })

  it('should register theme on init', async () => {
    const { default: devupUILoader } = await import('../loader')
    const t = {
      getOptions: () => ({
        package: 'package',
        cssDir: 'cssFile',
        watch: false,
        singleCss: true,
        theme: {
          colors: {
            primary: '#000',
          },
        },
        defaultClassMap: {
          button: 'button',
        },
        defaultFileMap: {
          button: 'button',
        },
        defaultSheet: {
          button: 'button',
        },
      }),
      async: vi.fn().mockReturnValue(vi.fn()),
      resourcePath: 'index.tsx',
      addDependency: vi.fn(),
    }
    devupUILoader.bind(t as any)(Buffer.from('code'), 'index.tsx')
    expect(registerTheme).toHaveBeenCalledTimes(1)
    expect(importClassMap).toHaveBeenCalledWith({
      button: 'button',
    })
    expect(importFileMap).toHaveBeenCalledWith({
      button: 'button',
    })
    expect(importSheet).toHaveBeenCalledWith({
      button: 'button',
    })

    devupUILoader.bind(t as any)(Buffer.from('code'), 'index.tsx')

    expect(registerTheme).toHaveBeenCalledTimes(1)
    expect(importClassMap).toHaveBeenCalledTimes(1)
    expect(importFileMap).toHaveBeenCalledTimes(1)
    expect(importSheet).toHaveBeenCalledTimes(1)
  })
})
