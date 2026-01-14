import * as fs from 'node:fs'
import * as path from 'node:path'

import * as wasm from '@devup-ui/wasm'
import {
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  mock,
  spyOn,
} from 'bun:test'
import * as tinyglobby from 'tinyglobby'

import * as findTopPackageRootModule from '../find-top-package-root'
import * as getPackageNameModule from '../get-package-name'
import * as hasLocalPackageModule from '../has-localpackage'
import { preload } from '../preload'

let readFileSyncSpy: ReturnType<typeof spyOn>
let writeFileSyncSpy: ReturnType<typeof spyOn>
let realpathSyncSpy: ReturnType<typeof spyOn>
let globSyncSpy: ReturnType<typeof spyOn>
let codeExtractSpy: ReturnType<typeof spyOn>
let getCssSpy: ReturnType<typeof spyOn>
let hasLocalPackageSpy: ReturnType<typeof spyOn>
let findTopPackageRootSpy: ReturnType<typeof spyOn>
let getPackageNameSpy: ReturnType<typeof spyOn>

beforeEach(() => {
  readFileSyncSpy = spyOn(fs, 'readFileSync').mockReturnValue('code')
  writeFileSyncSpy = spyOn(fs, 'writeFileSync').mockReturnValue(undefined)
  realpathSyncSpy = spyOn(fs, 'realpathSync').mockImplementation(
    (p) => p as string,
  )
  globSyncSpy = spyOn(tinyglobby, 'globSync').mockReturnValue([])
  codeExtractSpy = spyOn(wasm, 'codeExtract').mockReturnValue({
    code: 'extracted',
    css: 'css',
    cssFile: null,
    map: null,
    updatedBaseStyle: false,
    free: mock(),
    [Symbol.dispose]: mock(),
  } as any)
  getCssSpy = spyOn(wasm, 'getCss').mockReturnValue('global css')
  hasLocalPackageSpy = spyOn(
    hasLocalPackageModule,
    'hasLocalPackage',
  ).mockReturnValue(false)
  findTopPackageRootSpy = spyOn(
    findTopPackageRootModule,
    'findTopPackageRoot',
  ).mockReturnValue('/root')
  getPackageNameSpy = spyOn(
    getPackageNameModule,
    'getPackageName',
  ).mockReturnValue('test-pkg')
})

afterEach(() => {
  readFileSyncSpy.mockRestore()
  writeFileSyncSpy.mockRestore()
  realpathSyncSpy.mockRestore()
  globSyncSpy.mockRestore()
  codeExtractSpy.mockRestore()
  getCssSpy.mockRestore()
  hasLocalPackageSpy.mockRestore()
  findTopPackageRootSpy.mockRestore()
  getPackageNameSpy.mockRestore()
})

describe('preload', () => {
  it('should process files and write devup-ui.css', () => {
    globSyncSpy.mockReturnValue(['/project/src/index.tsx'])

    preload(/node_modules/, '@devup-ui/react', true, '/css', [], {}, '/project')

    expect(globSyncSpy).toHaveBeenCalled()
    expect(codeExtractSpy).toHaveBeenCalled()
    expect(writeFileSyncSpy).toHaveBeenCalledWith(
      path.join('/css', 'devup-ui.css'),
      'global css',
      'utf-8',
    )
  })

  it('should skip test files', () => {
    globSyncSpy.mockReturnValue([
      '/project/src/index.test.tsx',
      '/project/src/index.spec.ts',
      '/project/src/index.test-d.ts',
      '/project/src/types.d.ts',
    ])

    preload(/node_modules/, '@devup-ui/react', true, '/css', [], {}, '/project')

    expect(codeExtractSpy).not.toHaveBeenCalled()
  })

  it('should skip out and .next directories', () => {
    const cwd = process.cwd()
    realpathSyncSpy.mockImplementation((p) => p as string)
    globSyncSpy.mockReturnValue([
      path.join(cwd, 'out/index.tsx'),
      path.join(cwd, '.next/index.tsx'),
    ])

    preload(/node_modules/, '@devup-ui/react', true, '/css', [], {}, cwd)

    expect(codeExtractSpy).not.toHaveBeenCalled()
  })

  it('should skip files matching exclude regex', () => {
    globSyncSpy.mockReturnValue(['/project/node_modules/pkg/index.tsx'])

    preload(/node_modules/, '@devup-ui/react', true, '/css', [], {}, '/project')

    expect(codeExtractSpy).not.toHaveBeenCalled()
  })

  it('should write css file when cssFile is returned', () => {
    globSyncSpy.mockReturnValue(['/project/src/index.tsx'])
    codeExtractSpy.mockReturnValue({
      code: 'extracted',
      css: 'component css',
      cssFile: 'devup-ui-1.css',
      map: null,
      updatedBaseStyle: false,
      free: mock(),
      [Symbol.dispose]: mock(),
    } as any)

    preload(/node_modules/, '@devup-ui/react', true, '/css', [], {}, '/project')

    expect(writeFileSyncSpy).toHaveBeenCalledWith(
      path.join('/css', 'devup-ui-1.css'),
      'component css',
      'utf-8',
    )
  })

  it('should process local packages when include has items and hasLocalPackage', () => {
    hasLocalPackageSpy.mockReturnValue(true)
    getPackageNameSpy.mockReturnValue('@scope/local-pkg')

    // First call returns package.json paths, second returns source files
    globSyncSpy
      .mockReturnValueOnce(['/root/packages/local-pkg/package.json'])
      .mockReturnValueOnce(['/root/packages/local-pkg/src/index.tsx'])
      .mockReturnValueOnce(['/project/src/app.tsx'])

    preload(
      /node_modules/,
      '@devup-ui/react',
      true,
      '/css',
      ['@scope/local-pkg'],
      {},
      '/project',
    )

    expect(findTopPackageRootSpy).toHaveBeenCalled()
    expect(getPackageNameSpy).toHaveBeenCalledWith(
      '/root/packages/local-pkg/package.json',
    )
  })

  it('should not process local packages when nested is true', () => {
    hasLocalPackageSpy.mockReturnValue(true)
    getPackageNameSpy.mockReturnValue('@scope/local-pkg')
    globSyncSpy
      .mockReturnValueOnce(['/root/packages/local-pkg/package.json'])
      .mockReturnValueOnce([])

    preload(
      /node_modules/,
      '@devup-ui/react',
      true,
      '/css',
      ['@scope/local-pkg'],
      {},
      '/project',
      true, // nested = true
    )

    // Should return early when nested is true after processing local packages
    expect(writeFileSyncSpy).not.toHaveBeenCalled()
  })

  it('should handle empty css value', () => {
    globSyncSpy.mockReturnValue(['/project/src/index.tsx'])
    codeExtractSpy.mockReturnValue({
      code: 'extracted',
      css: undefined,
      cssFile: 'devup-ui-1.css',
      map: null,
      updatedBaseStyle: false,
      free: mock(),
      [Symbol.dispose]: mock(),
    } as any)

    preload(/node_modules/, '@devup-ui/react', true, '/css', [], '/project')

    expect(writeFileSyncSpy).toHaveBeenCalledWith(
      path.join('/css', 'devup-ui-1.css'),
      '',
      'utf-8',
    )
  })
})
