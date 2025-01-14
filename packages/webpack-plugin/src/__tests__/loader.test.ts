import { writeFileSync } from 'node:fs'

import { codeExtract } from '@devup-ui/wasm'

import devupUILoader from '../loader'

vi.mock('@devup-ui/wasm')
vi.mock('node:fs')

beforeEach(() => {
  vi.resetAllMocks()
})

describe('devupUILoader', () => {
  it('should ignore lib files', () => {
    const t = {
      getOptions: () => ({
        package: 'package',
        cssFile: 'cssFile',
      }),
      async: vi.fn().mockReturnValue(vi.fn()),
      resourcePath: 'node_modules/package/index.ts',
    }
    devupUILoader.bind(t as any)(
      Buffer.from('code'),
      'node_modules/package/index.ts',
    )

    expect(t.async).toHaveBeenCalled()
    expect(t.async()).toHaveBeenCalledWith(null, Buffer.from('code'))
  })

  it('should ignore wrong files', () => {
    const t = {
      getOptions: () => ({
        package: 'package',
        cssFile: 'cssFile',
      }),
      async: vi.fn().mockReturnValue(vi.fn()),
      resourcePath: 'node_modules/package/index.css',
    }
    devupUILoader.bind(t as any)(
      Buffer.from('code'),
      'node_modules/package/index.css',
    )

    expect(t.async).toHaveBeenCalled()
    expect(t.async()).toHaveBeenCalledWith(null, Buffer.from('code'))
  })

  it('should extract code with css', () => {
    const t = {
      getOptions: () => ({
        package: 'package',
        cssFile: 'cssFile',
      }),
      async: vi.fn().mockReturnValue(vi.fn()),
      resourcePath: 'index.tsx',
    }
    vi.mocked(codeExtract).mockReturnValue({
      code: 'code',
      css: 'css',
      free: vi.fn(),
    })
    devupUILoader.bind(t as any)(Buffer.from('code'), 'index.tsx')

    expect(t.async).toHaveBeenCalled()
    expect(codeExtract).toHaveBeenCalledWith(
      'index.tsx',
      'code',
      'package',
      'cssFile',
    )
    expect(t.async()).toHaveBeenCalledWith(null, 'code')
    expect(writeFileSync).toHaveBeenCalledWith('cssFile', 'css', {
      encoding: 'utf-8',
    })
  })

  it('should extract code without css', () => {
    const t = {
      getOptions: () => ({
        package: 'package',
        cssFile: 'cssFile',
      }),
      async: vi.fn().mockReturnValue(vi.fn()),
      resourcePath: 'index.tsx',
    }
    vi.mocked(codeExtract).mockReturnValue({
      code: 'code',
      css: undefined,
      free: vi.fn(),
    })
    devupUILoader.bind(t as any)(Buffer.from('code'), 'index.tsx')

    expect(t.async).toHaveBeenCalled()
    expect(codeExtract).toHaveBeenCalledWith(
      'index.tsx',
      'code',
      'package',
      'cssFile',
    )
    expect(t.async()).toHaveBeenCalledWith(null, 'code')
    expect(writeFileSync).not.toHaveBeenCalledWith('cssFile', 'css', {
      encoding: 'utf-8',
    })
  })

  it('should handle error', () => {
    const t = {
      getOptions: () => ({
        package: 'package',
        cssFile: 'cssFile',
      }),
      async: vi.fn().mockReturnValue(vi.fn()),
      resourcePath: 'index.tsx',
    }
    vi.mocked(codeExtract).mockImplementation(() => {
      throw new Error('error')
    })
    devupUILoader.bind(t as any)(Buffer.from('code'), 'index.tsx')

    expect(t.async).toHaveBeenCalled()
    expect(t.async()).toHaveBeenCalledWith(new Error('error'))
  })
})
