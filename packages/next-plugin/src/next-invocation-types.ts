export type NextInvocationDecision =
  | {
      readonly kind: 'receive'
      readonly authority: 'adapter' | 'webpack-beforeCompile'
      readonly isolation: 'current-isolate'
    }
  | {
      readonly kind: 'handoff'
      readonly target: 'turbopack-build-thread' | 'webpack-build-process'
      readonly prepared: false
    }
  | {
      readonly kind: 'inert'
      readonly reason:
        | 'unneeded'
        | 'wrapper'
        | 'cache'
        | 'supplied-config'
        | 'telemetry'
        | 'non-compiling'
    }

export interface NextInvocationRuntime {
  readonly entryPath: string | undefined
  readonly argv: readonly string[]
  readonly isMainThread: boolean
  readonly hasIpc: boolean
  readonly loadedModules: readonly string[]
  readonly env: {
    readonly TURBOPACK?: string
    readonly NEXT_RSPACK?: string
    readonly __NEXT_DEV_SERVER?: string
    readonly NEXT_PRIVATE_WORKER?: string
    readonly NEXT_PRIVATE_BUILD_WORKER?: string
    readonly IS_NEXT_WORKER?: string
    readonly NEXT_TURBOPACK_USE_WORKER?: string
  }
}

export interface NextInvocationInput {
  readonly required: boolean
  readonly stage:
    | 'wrapper'
    | 'adapter'
    | 'cache'
    | 'supplied-config'
    | 'webpack-beforeCompile'
  readonly installed: {
    readonly version: string
    readonly packageDir: string
  }
  readonly evaluation: {
    readonly phase: string
    readonly projectDir: string
    readonly expectedProjectDir: string
    readonly configFile: string
  }
  readonly runtime: NextInvocationRuntime
  readonly build:
    | { readonly projectDir: string; readonly mode: string | undefined }
    | undefined
  readonly config: {
    readonly webpackBuildWorker: boolean | undefined
    readonly hasWebpack: boolean
  }
}
