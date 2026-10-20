import type { Configuration } from 'webpack'

export type SlotMap = Readonly<Record<string, number>>
export type Snapshot = {
  readonly classes: Readonly<Record<string, SlotMap>>
  readonly files: SlotMap
  readonly residual: number
  readonly css: string
}
export type Row = readonly [string, string, string]
export type Aliases = Readonly<Record<string, string | null>>
export type ReserveArgs = [
  readonly Row[],
  string,
  boolean,
  boolean,
  boolean,
  Aliases,
]
export type ExtractArgs = [
  string,
  string,
  string,
  string,
  boolean,
  boolean,
  boolean,
  Aliases,
]
export type ExtractOutput = {
  readonly code: string
  readonly css: string | undefined
  readonly cssFile: string | undefined
  readonly map: string | undefined
  readonly dependencies: readonly string[]
  readonly updatedBaseStyle: boolean
}
export type Payload =
  | { readonly kind: 'reset'; readonly state: Snapshot }
  | {
      readonly kind: 'reserve'
      readonly args: ReserveArgs
      readonly before: Snapshot
      readonly after: Snapshot
    }
  | {
      readonly kind: 'extract-start'
      readonly args: ExtractArgs
      readonly before: number
    }
  | {
      readonly kind: 'extract'
      readonly args: ExtractArgs
      readonly before: number
      readonly after: number
      readonly output: ExtractOutput
    }
  | {
      readonly kind: 'options'
      readonly compilerRole: string
      readonly dev: boolean
      readonly buildId: string
      readonly delegation: number
    }
  | { readonly kind: 'compilation'; readonly compilerName: string | undefined }
  | {
      readonly kind: 'done'
      readonly errors: boolean
      readonly state: Snapshot
    }
export type NativeEvent = Payload & {
  readonly pid: number
  readonly epoch: number
  readonly phase: string
  readonly role: string
}
export type NativeConfig = Configuration & {
  plugins: NonNullable<Configuration['plugins']>
}
