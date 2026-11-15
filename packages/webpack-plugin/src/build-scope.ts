import {
  BuildGeneration,
  type BuildIntegration,
  buildOperationSite,
  type GenerationSite,
  runBuildOperation,
} from '@devup-ui/plugin-utils'
import {
  exportCanonicalMap,
  exportClassMap,
  exportFileMap,
  exportReservationState,
  exportSheet,
  ownership,
} from '@devup-ui/wasm'

/** @internal The public WASM exports own these JSON schemas. */
interface SerializedBuildState {
  readonly sheet: string
  readonly classes: string
  readonly files: string
  readonly canonical: string
  readonly reservation: string
}

/** @internal No owner is accepted through user-facing plugin options. */
export interface WebpackGenerationBinding {
  readonly owner: BuildGeneration<SerializedBuildState>
  readonly complete: boolean
}

const engine = {
  ownership,
  capture(): SerializedBuildState {
    return {
      sheet: exportSheet(),
      classes: exportClassMap(),
      files: exportFileMap(),
      canonical: exportCanonicalMap(),
      reservation: exportReservationState(),
    }
  },
}

/** @internal Minted by the integration, never inferred from paths or caches. */
export function createWebpackGeneration(): BuildGeneration<SerializedBuildState> {
  return new BuildGeneration<SerializedBuildState>()
}

export class WebpackBuildScope {
  private readonly site = buildOperationSite('configure Webpack')
  private configure: () => void = () => undefined
  private release: (() => void) | undefined
  private closed = false

  constructor(
    private readonly binding: WebpackGenerationBinding,
    private readonly standalone: boolean,
    private readonly context: BuildIntegration = {
      integration: 'Webpack',
      root: process.cwd(),
    },
  ) {}

  setConfiguration(configure: () => void): void {
    this.configure = configure
  }

  run<T>(action: () => T, site = this.site): T {
    return runBuildOperation(this.context, () =>
      this.binding.owner.run(engine, {
        site,
        configure: this.configure,
        action,
      }),
    )
  }

  stylesheet<T>(file: string, action: () => T): T {
    return this.run(action, {
      file,
      line: 1,
      column: 1,
      operation: 'stylesheet',
    })
  }

  start(): void {
    this.release ??= this.binding.owner.acquire(this.binding.complete)
  }

  close(): void {
    if (this.closed) return
    this.closed = true
    this.release?.()
    this.release = undefined
    if (this.standalone) this.binding.owner.dispose()
    else if (!this.binding.owner.disposed) this.binding.owner.clear()
  }

  abort(): void {
    this.binding.owner.dispose()
    this.close()
  }
}

declare global {
  // Bundled entry/loader modules share only weak compiler identity, never paths.
  var __devupUiBuildScopes: WeakMap<object, WebpackBuildScope> | undefined
}

export function bindCompilerScope(
  compiler: object,
  binding?: WebpackGenerationBinding,
  context?: BuildIntegration,
): WebpackBuildScope {
  const selected =
    binding && !binding.owner.disposed
      ? binding
      : { owner: createWebpackGeneration(), complete: true }
  const scope = new WebpackBuildScope(selected, selected !== binding, context)
  ;(globalThis.__devupUiBuildScopes ??= new WeakMap()).set(compiler, scope)
  return scope
}

interface CompilerIdentity {
  readonly root?: object
}

export function compilerScope(
  compiler: CompilerIdentity | undefined,
): WebpackBuildScope | undefined {
  return compiler === undefined
    ? undefined
    : (globalThis.__devupUiBuildScopes?.get(compiler) ??
        (compiler.root
          ? globalThis.__devupUiBuildScopes?.get(compiler.root)
          : undefined))
}

export function withCompilerScope<T>(
  compiler: CompilerIdentity | undefined,
  action: () => T,
  site?: GenerationSite,
): T {
  const scope = compilerScope(compiler)
  return scope ? scope.run(action, site) : action()
}
