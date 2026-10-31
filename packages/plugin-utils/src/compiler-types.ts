export type DevupEngine = 'wasm' | 'native'

type NativeCompiler = typeof import('@devup-ui/binding')

/** The public functions, not the backend-specific Output constructor. */
export type DevupCompiler = Pick<
  NativeCompiler,
  Exclude<keyof NativeCompiler, 'Output'>
>

/** Output ownership stays with the caller; release it after reading its getters. */
export type DevupCompilerOutput = Pick<
  import('@devup-ui/binding').Output,
  | 'code'
  | 'css'
  | 'cssFile'
  | 'map'
  | 'dependencies'
  | 'updatedBaseStyle'
  | 'free'
  | typeof Symbol.dispose
>
