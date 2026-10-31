import { realpathSync } from 'node:fs'
import { createRequire } from 'node:module'
import { isAbsolute } from 'node:path'
import { fileURLToPath } from 'node:url'

import type { DevupCompiler, DevupEngine } from './compiler-types'
import { nativePlatformPackage } from './native-platform'

interface CompilerLoaders {
  readonly native: () => typeof import('@devup-ui/binding')
  readonly wasm: () => typeof import('@devup-ui/wasm')
  readonly platform: () => string
}

export class InvalidEngineError extends TypeError {
  readonly name = 'InvalidEngineError'

  constructor(readonly engine: unknown) {
    super("devup-ui engine must be 'native' or 'wasm'")
  }
}

export class NativeCompilerLoadError extends Error {
  readonly name = 'NativeCompilerLoadError'

  constructor(
    readonly platformPackage: string,
    cause: unknown,
  ) {
    super(
      `Cannot load ${platformPackage}. Install its optional dependency and native libraries, or use engine: 'wasm'.`,
      { cause },
    )
  }
}

/** @internal Concrete load boundary for source tests; not exported by the barrel. */
export function createCompilerSelector(loaders: CompilerLoaders) {
  return (engine: unknown = undefined): DevupCompiler => {
    switch (engine) {
      case 'wasm':
        return loaders.wasm()
      case undefined:
      case 'native': {
        const platformPackage = loaders.platform()
        try {
          return loaders.native()
        } catch (cause) {
          throw new NativeCompilerLoadError(platformPackage, cause)
        }
      }
      default:
        throw new InvalidEngineError(engine)
    }
  }
}

interface RealpathFunctions {
  readonly native: (filename: string) => string
  readonly regular: (filename: string) => string
}

/** @internal Concrete path boundary for source tests; not exported by the barrel. */
export function compilerRealpath(
  filename: string,
  realpath: RealpathFunctions = {
    native: realpathSync.native,
    regular: realpathSync,
  },
): string {
  try {
    return realpath.native(filename)
  } catch (error) {
    if (
      typeof error === 'object' &&
      error !== null &&
      'code' in error &&
      error.code === 'EISDIR'
    ) {
      return realpath.regular(filename)
    }
    throw error
  }
}

function physicalRequire() {
  // CJS build substitutes module.filename; ESM keeps its own file URL.
  const location = import.meta.url
  const filename = isAbsolute(location) ? location : fileURLToPath(location)
  return createRequire(compilerRealpath(filename))
}

const selectCompiler = createCompilerSelector({
  native() {
    const compiler: typeof import('@devup-ui/binding') =
      physicalRequire()('@devup-ui/binding')
    return compiler
  },
  wasm() {
    const compiler: typeof import('@devup-ui/wasm') =
      physicalRequire()('@devup-ui/wasm')
    return compiler
  },
  platform() {
    return nativePlatformPackage({
      platform: process.platform,
      arch: process.arch,
      report: process.report.getReport,
    })
  },
})

/** Select synchronously without resolving or initializing the unselected engine. */
export function loadCompiler(engine?: DevupEngine): DevupCompiler {
  return selectCompiler(engine)
}
