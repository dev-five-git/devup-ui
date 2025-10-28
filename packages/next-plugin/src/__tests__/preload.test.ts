import { realpathSync, writeFileSync } from 'node:fs'
import { readFileSync } from 'node:fs'
import { existsSync } from 'node:fs'
import { join } from 'node:path'

import { codeExtract, registerTheme } from '@devup-ui/wasm'
import { globSync } from 'glob'

import { preload } from '../preload'

// Mock dependencies
vi.mock('node:fs')
vi.mock('@devup-ui/wasm')
vi.mock('glob')

// Mock globSync
vi.mock('node:fs', () => ({
  readFileSync: vi.fn(),
  writeFileSync: vi.fn(),
  mkdirSync: vi.fn(),
  existsSync: vi.fn(),
  realpathSync: vi.fn().mockReturnValue('src/App.tsx'),
}))

vi.mock('glob', () => ({
  globSync: vi.fn(),
}))

// Mock @devup-ui/wasm
vi.mock('@devup-ui/wasm', () => ({
  codeExtract: vi.fn(),
  registerTheme: vi.fn(),
}))

describe('preload', () => {
  beforeEach(() => {
    vi.clearAllMocks()

    // Default mock implementations
    vi.mocked(globSync).mockReturnValue([
      'src/App.tsx',
      'src/components/Button.tsx',
    ])
    vi.mocked(readFileSync).mockReturnValue(
      'const Button = () => <div>Hello</div>',
    )
    vi.mocked(codeExtract).mockReturnValue({
      free: vi.fn(),
      cssFile: 'styles.css',
      css: '.button { color: red; }',
      code: '',
      map: '',
      updatedBaseStyle: false,
      [Symbol.dispose]: vi.fn(),
    })
    vi.mocked(existsSync).mockReturnValue(true)
  })

  it('should find project root and collect files', () => {
    const excludeRegex = /node_modules/
    const libPackage = '@devup-ui/react'
    const singleCss = false
    const theme = { colors: { primary: 'blue' } }
    const cssDir = '/output/css'

    preload(excludeRegex, libPackage, singleCss, theme, cssDir)

    expect(globSync).toHaveBeenCalledWith(
      ['**/*.tsx', '**/*.ts', '**/*.js', '**/*.mjs'],
      {
        follow: true,
        absolute: true,
      },
    )
  })

  it('should register theme before processing files', () => {
    const theme = { colors: { primary: 'blue' } }

    preload(/node_modules/, '@devup-ui/react', false, theme, '/output/css')

    expect(registerTheme).toHaveBeenCalledWith(theme)
  })

  it('should process each collected file', () => {
    const files = ['src/App.tsx', 'src/components/Button.tsx', '.next/page.tsx']
    vi.mocked(globSync).mockReturnValue(files)
    vi.mocked(realpathSync)
      .mockReturnValueOnce('src/App.tsx')
      .mockReturnValueOnce('src/components/Button.tsx')
      .mockReturnValueOnce('.next/page.tsx')
    preload(/node_modules/, '@devup-ui/react', false, {}, '/output/css')

    expect(codeExtract).toHaveBeenCalledTimes(2)
    expect(codeExtract).toHaveBeenCalledWith(
      expect.stringMatching(/App\.tsx$/),
      'const Button = () => <div>Hello</div>',
      '@devup-ui/react',
      '/output/css',
      false,
      false,
      true,
    )
  })

  it('should write CSS file when cssFile is returned', () => {
    vi.mocked(codeExtract).mockReturnValue({
      cssFile: 'styles.css',
      css: '.button { color: red; }',
      free: vi.fn(),
      code: '',
      map: '',
      updatedBaseStyle: false,
      [Symbol.dispose]: vi.fn(),
    })

    preload(/node_modules/, '@devup-ui/react', false, {}, '/output/css')

    expect(writeFileSync).toHaveBeenCalledWith(
      join('/output/css', 'styles.css'),
      '.button { color: red; }',
      'utf-8',
    )
  })

  it('should not write CSS file when cssFile is null', () => {
    vi.mocked(codeExtract).mockReturnValue({
      cssFile: undefined,
      css: '.button { color: red; }',
      free: vi.fn(),
      code: '',
      map: '',
      updatedBaseStyle: false,
      [Symbol.dispose]: vi.fn(),
    })

    preload(/node_modules/, '@devup-ui/react', false, {}, '/output/css')

    expect(writeFileSync).not.toHaveBeenCalled()
  })

  it('should handle empty CSS content', () => {
    vi.mocked(codeExtract).mockReturnValue({
      cssFile: 'styles.css',
      css: '',
      free: vi.fn(),
      code: '',
      map: '',
      updatedBaseStyle: false,
      [Symbol.dispose]: vi.fn(),
    })

    preload(/node_modules/, '@devup-ui/react', false, {}, '/output/css')

    expect(writeFileSync).toHaveBeenCalledWith(
      join('/output/css', 'styles.css'),
      '',
      'utf-8',
    )
  })

  it('should handle undefined CSS content', () => {
    vi.mocked(codeExtract).mockReturnValue({
      cssFile: 'styles.css',
      css: undefined,
      free: vi.fn(),
      code: '',
      map: '',
      updatedBaseStyle: false,
      [Symbol.dispose]: vi.fn(),
    })

    preload(/node_modules/, '@devup-ui/react', false, {}, '/output/css')

    expect(writeFileSync).toHaveBeenCalledWith(
      join('/output/css', 'styles.css'),
      '',
      'utf-8',
    )
  })

  it('should pass correct parameters to codeExtract', () => {
    const libPackage = '@devup-ui/react'
    const singleCss = true
    const cssDir = '/custom/css/dir'

    preload(/node_modules/, libPackage, singleCss, {}, cssDir)

    expect(codeExtract).toHaveBeenCalledWith(
      expect.stringMatching(/App\.tsx$/),
      'const Button = () => <div>Hello</div>',
      libPackage,
      cssDir,
      singleCss,
      false,
      true,
    )
  })

  it('should handle multiple files with different CSS outputs', () => {
    const files = ['src/App.tsx', 'src/components/Button.tsx']
    vi.mocked(globSync).mockReturnValue(files)

    vi.mocked(codeExtract)
      .mockReturnValueOnce({
        cssFile: 'app.css',
        css: '.app { margin: 0; }',
        free: vi.fn(),
        code: '',
        map: '',
        updatedBaseStyle: false,
        [Symbol.dispose]: vi.fn(),
      })
      .mockReturnValueOnce({
        free: vi.fn(),
        cssFile: 'button.css',
        css: '.button { color: blue; }',
        code: '',
        map: '',
        updatedBaseStyle: false,
        [Symbol.dispose]: vi.fn(),
      })

    preload(/node_modules/, '@devup-ui/react', false, {}, '/output/css')

    expect(writeFileSync).toHaveBeenCalledTimes(2)
    expect(writeFileSync).toHaveBeenCalledWith(
      join('/output/css', 'app.css'),
      '.app { margin: 0; }',
      'utf-8',
    )
    expect(writeFileSync).toHaveBeenCalledWith(
      join('/output/css', 'button.css'),
      '.button { color: blue; }',
      'utf-8',
    )
  })
})
