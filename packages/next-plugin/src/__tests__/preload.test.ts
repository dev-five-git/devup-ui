import { realpathSync, writeFileSync } from 'node:fs'
import { readFileSync } from 'node:fs'
import { existsSync } from 'node:fs'
import { join } from 'node:path'

import { codeExtract, getCss } from '@devup-ui/wasm'
import { globSync } from 'tinyglobby'

import { findTopPackageRoot } from '../find-top-package-root'
import { getPackageName } from '../get-package-name'
import { hasLocalPackage } from '../has-localpackage'
import { preload } from '../preload'

// Mock dependencies
vi.mock('node:fs')
vi.mock('@devup-ui/wasm')
vi.mock('tinyglobby')

// Mock globSync
vi.mock('node:fs', () => ({
  readFileSync: vi.fn(),
  writeFileSync: vi.fn(),
  mkdirSync: vi.fn(),
  existsSync: vi.fn(),
  realpathSync: vi.fn().mockReturnValue('src/App.tsx'),
}))

vi.mock('tinyglobby', () => ({
  globSync: vi.fn(),
}))

// Mock @devup-ui/wasm
vi.mock('@devup-ui/wasm', () => ({
  codeExtract: vi.fn(),
  registerTheme: vi.fn(),
  getCss: vi.fn(),
}))

vi.mock('../find-top-package-root', () => ({
  findTopPackageRoot: vi.fn(),
}))

vi.mock('../get-package-name', () => ({
  getPackageName: vi.fn(),
}))

vi.mock('../has-localpackage', () => ({
  hasLocalPackage: vi.fn(),
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
    const cssDir = '/output/css'

    preload(excludeRegex, libPackage, singleCss, cssDir, [])

    expect(globSync).toHaveBeenCalledWith(
      ['**/*.tsx', '**/*.ts', '**/*.js', '**/*.mjs'],
      {
        followSymbolicLinks: true,
        absolute: true,
        cwd: expect.any(String),
      },
    )
  })

  it('should process each collected file', () => {
    const files = ['src/App.tsx', 'src/components/Button.tsx', '.next/page.tsx']
    vi.mocked(globSync).mockReturnValue(files)
    vi.mocked(realpathSync)
      .mockReturnValueOnce('src/App.tsx')
      .mockReturnValueOnce('src/components/Button.tsx')
      .mockReturnValueOnce('.next/page.tsx')
    preload(/node_modules/, '@devup-ui/react', false, '/output/css', [])

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

    preload(/node_modules/, '@devup-ui/react', false, '/output/css', [])

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
    vi.mocked(getCss).mockReturnValue('')

    preload(/node_modules/, '@devup-ui/react', false, '/output/css', [])

    expect(writeFileSync).toHaveBeenCalledWith(
      join('/output/css', 'devup-ui.css'),
      '',
      'utf-8',
    )
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

    preload(/node_modules/, '@devup-ui/react', false, '/output/css', [])

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

    preload(/node_modules/, '@devup-ui/react', false, '/output/css', [])

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

    preload(/node_modules/, libPackage, singleCss, cssDir, [])

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

    preload(/node_modules/, '@devup-ui/react', false, '/output/css', [])

    expect(writeFileSync).toHaveBeenCalledTimes(3)
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

  it('should recurse into local workspaces when include is provided', () => {
    const files = ['src/App.tsx']
    vi.mocked(findTopPackageRoot).mockReturnValue('/repo')
    vi.mocked(hasLocalPackage)
      .mockReturnValueOnce(true)
      .mockReturnValueOnce(false)
    vi.mocked(globSync)
      .mockReturnValueOnce([
        '/repo/packages/pkg-a/package.json',
        '/repo/packages/pkg-b/package.json',
      ])
      .mockReturnValueOnce(files)
    vi.mocked(getPackageName)
      .mockReturnValueOnce('pkg-a')
      .mockReturnValueOnce('pkg-b')
    vi.mocked(realpathSync).mockReturnValueOnce('src/App.tsx')

    preload(/node_modules/, '@devup-ui/react', false, '/output/css', ['pkg-a'])

    expect(findTopPackageRoot).toHaveBeenCalled()
    expect(globSync).toHaveBeenCalledWith(
      ['package.json', '!**/node_modules/**'],
      {
        followSymbolicLinks: true,
        absolute: true,
        cwd: '/repo',
      },
    )
    expect(codeExtract).toHaveBeenCalledTimes(3)
    expect(realpathSync).toHaveBeenCalledWith('src/App.tsx')
  })

  it('should skip test and build outputs based on filters', () => {
    vi.mocked(globSync).mockReturnValue([
      'src/App.test.tsx',
      '.next/page.tsx',
      'out/index.js',
      'src/keep.ts',
    ])
    vi.mocked(realpathSync)
      .mockReturnValueOnce('src/App.test.tsx')
      .mockReturnValueOnce('.next/page.tsx')
      .mockReturnValueOnce('out/index.js')
      .mockReturnValueOnce('src/keep.ts')

    preload(/exclude/, '@devup-ui/react', false, '/output/css', [])

    expect(codeExtract).toHaveBeenCalledTimes(1)
    expect(codeExtract).toHaveBeenCalledWith(
      expect.stringMatching(/keep\.ts$/),
      'const Button = () => <div>Hello</div>',
      '@devup-ui/react',
      '/output/css',
      false,
      false,
      true,
    )
  })

  it('should return early when nested is true and include packages exist', () => {
    vi.mocked(findTopPackageRoot).mockReturnValue('/repo')
    vi.mocked(hasLocalPackage).mockReturnValue(true)
    // Return empty array so no recursive calls happen, but include.length > 0 check passes
    vi.mocked(globSync).mockReturnValue([])
    vi.mocked(getPackageName).mockReturnValue('pkg-a')

    // Call with nested = true (7th parameter)
    preload(
      /node_modules/,
      '@devup-ui/react',
      false,
      '/output/css',
      ['pkg-a'],
      '/some/path',
      true,
    )

    // When nested is true, it should return early after processing includes
    // and not write the final devup-ui.css file
    expect(writeFileSync).not.toHaveBeenCalledWith(
      expect.stringContaining('devup-ui.css'),
      expect.any(String),
      'utf-8',
    )
  })
})
