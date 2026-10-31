import * as native from '@devup-ui/binding'
import * as wasm from '@devup-ui/wasm'
import { describe, expect, it } from 'bun:test'

import {
  compilerRealpath,
  createCompilerSelector,
  InvalidEngineError,
  loadCompiler,
} from '../compiler'
import type {
  DevupCompiler,
  DevupCompilerOutput,
  DevupEngine,
} from '../compiler-types'
import type { DevupUIBasePluginOptions } from '../shared'

const nativeCompiler: DevupCompiler = native
const wasmCompiler: DevupCompiler = wasm
type Accepted<T> = T extends DevupEngine ? true : false
type Resolver = NonNullable<Parameters<DevupCompiler['setModuleResolver']>[0]>
const optionProof: Pick<DevupUIBasePluginOptions, 'engine'> = {}
const engineProof: [Accepted<'native'>, Accepted<'wasm'>, Accepted<'other'>] = [
  true,
  true,
  false,
]
const constructorProof: 'Output' extends keyof DevupCompiler ? true : false =
  false
const resolverProof: ((
  specifier: string,
  importer: string,
) => number) extends Resolver
  ? true
  : false = false

describe('compiler structural types', () => {
  it('keeps literal options optional and resolver results precise', () => {
    // Given / When: these assignments are also checked by test:types.
    const configured: Pick<DevupUIBasePluginOptions, 'engine'> = {
      engine: 'native',
    }
    // Then
    expect([
      optionProof,
      configured,
      engineProof,
      constructorProof,
      resolverProof,
    ]).toEqual([{}, { engine: 'native' }, [true, true, false], false, false])
  })

  for (const [name, compiler] of [
    ['native', nativeCompiler],
    ['wasm', wasmCompiler],
  ] as const) {
    it(`accepts the actual ${name} namespace and releases its structural output`, () => {
      // Given
      compiler.resetStateForTesting()
      try {
        // When: both concrete return types must assign without a constructor cast.
        using output: DevupCompilerOutput = compiler.codeExtract(
          'compiler-type.tsx',
          "import {Box} from '@devup-ui/react'; export const view=<Box p={3}/>;",
          '@devup-ui/react',
          'df',
          true,
          false,
          false,
          {},
        )
        // Then
        expect(output.code).toContain('className=')
        expect(output.css).toContain('padding:12px')
        expect(output.dependencies).toEqual([])
        expect(output.map).toBeDefined()
        expect(
          Object.keys(compiler).filter(
            (key) => key !== 'Output' && key !== 'default',
          ),
        ).toHaveLength(28)
      } finally {
        compiler.resetStateForTesting()
      }
    })
  }
})

// These assignments check the concrete extraction types, independently of the facade.
const nativeOutput: (
  output: ReturnType<typeof native.codeExtract>,
) => DevupCompilerOutput = (output) => output
const wasmOutput: (
  output: ReturnType<typeof wasm.codeExtract>,
) => DevupCompilerOutput = (output) => output

for (const [name, compiler, accept] of [
  ['native', native, nativeOutput],
  ['wasm', wasm, wasmOutput],
] as const) {
  it(`accepts the concrete ${name} no-map output without retaining it`, () => {
    // Given
    compiler.resetStateForTesting()
    try {
      // When
      using output = compiler.codeExtractWithoutSourceMap(
        'plain.ts',
        'export const n=1',
        '@devup-ui/react',
        'df',
        true,
        false,
        false,
        {},
      )
      // Then
      expect(accept(output).map).toBeUndefined()
    } finally {
      compiler.resetStateForTesting()
    }
  })
}

describe('compiler physical path', () => {
  it('falls back once on the same path when native realpath throws EISDIR', () => {
    // Given
    const filename = 'compiler.js'
    const nativePaths: string[] = []
    const regularPaths: string[] = []
    // When
    const resolved = compilerRealpath(filename, {
      native(path) {
        nativePaths.push(path)
        throw Object.assign(new Error('RAM disk'), { code: 'EISDIR' })
      },
      regular(path) {
        regularPaths.push(path)
        return 'physical/compiler.js'
      },
    })
    // Then
    expect(resolved).toBe('physical/compiler.js')
    expect(nativePaths).toEqual([filename])
    expect(regularPaths).toEqual([filename])
  })

  it.each([
    Object.assign(new Error('missing path'), { code: 'ENOENT' }),
    new Error('unknown failure'),
    { thrown: 'value' },
    null,
    undefined,
    'failure',
  ])('rethrows %s unchanged without regular realpath', (failure) => {
    // Given
    expect.assertions(2)
    let fallback = 0
    // When
    try {
      compilerRealpath('compiler.js', {
        native() {
          throw failure
        },
        regular() {
          fallback++
          return 'unexpected'
        },
      })
    } catch (error) {
      // Then
      expect(error).toBe(failure)
    }
    expect(fallback).toBe(0)
  })
})

describe('compiler selection contract', () => {
  it.each([undefined, 'native', 'wasm'] as const)(
    'selects the actual namespace for %s',
    (engine) => {
      // Given / When
      const compiler = loadCompiler(engine)
      // Then
      expect(compiler.codeExtract).toBe(
        engine === 'wasm' ? wasm.codeExtract : native.codeExtract,
      )
    },
  )

  it.each([null, '', false, 0, 'other', {}, Symbol('invalid')])(
    'rejects %s before any host or backend access',
    (engine) => {
      // Given
      let accesses = 0
      const fail = (): never => {
        accesses++
        throw new Error('unexpected access')
      }
      const select = createCompilerSelector({
        native: fail,
        wasm: fail,
        platform: fail,
      })
      // When / Then
      expect(() => select(engine)).toThrow(InvalidEngineError)
      expect(accesses).toBe(0)
    },
  )

  it('retains the earlier namespace when the other engine is selected', () => {
    // Given
    const select = createCompilerSelector({
      native: () => native,
      wasm: () => wasm,
      platform: () => '@devup-ui/binding-test',
    })
    const previous = select('wasm')
    // When
    const next = select()
    // Then
    expect(previous).toBe(wasm)
    expect(next).toBe(native)
    expect(select('wasm')).toBe(previous)
  })

  for (const cause of [
    new Error('corrupt image'),
    Object.assign(new Error('transitive library'), {
      code: 'MODULE_NOT_FOUND',
    }),
    { thrown: 'value' },
  ]) {
    it('retains the exact native failure and escape without fallback', () => {
      // Given
      let fallback = 0
      const platformPackage = '@devup-ui/binding-linux-x64-gnu'
      const select = createCompilerSelector({
        native() {
          throw cause
        },
        wasm() {
          fallback++
          return wasm
        },
        platform: () => platformPackage,
      })
      // When / Then
      try {
        select('native')
        throw new Error('native failure was swallowed')
      } catch (error) {
        if (!(error instanceof Error)) throw error
        expect(error.cause).toBe(cause)
        expect(error.message).toContain(platformPackage)
        expect(error.message).toContain("engine: 'wasm'")
      }
      expect(fallback).toBe(0)
    })
  }

  it('propagates host failure before native loading', () => {
    // Given
    const failure = new Error('unknown libc')
    let loads = 0
    const select = createCompilerSelector({
      native() {
        loads++
        return native
      },
      wasm: () => wasm,
      platform() {
        throw failure
      },
    })
    // When / Then
    expect(() => select()).toThrow(failure)
    expect(loads).toBe(0)
  })

  it('preserves a WASM load failure without probing native context', () => {
    // Given
    const failure = new Error('wasm unavailable')
    const select = createCompilerSelector({
      native: () => native,
      wasm() {
        throw failure
      },
      platform() {
        throw new Error('unexpected host probe')
      },
    })
    // When / Then
    expect(() => select('wasm')).toThrow(failure)
  })
})
