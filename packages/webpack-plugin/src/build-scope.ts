import {
  BuildGeneration,
  type BuildIntegration,
  resetOwnedBuildState,
  runBuildOperation,
} from '@devup-ui/plugin-utils'
import {
  exportCanonicalMap,
  exportClassMap,
  exportFileMap,
  exportSheet,
  importCanonicalMap,
  importClassMap,
  importFileMap,
  importSheet,
  resetBuildState,
} from '@devup-ui/wasm'

/** @internal The public WASM exports own these JSON schemas. */
interface SerializedBuildState {
  readonly sheet: string
  readonly classes: string
  readonly files: string
  readonly canonical: string
}

/** @internal No owner is accepted through user-facing plugin options. */
export interface WebpackGenerationBinding {
  readonly owner: BuildGeneration<SerializedBuildState>
  readonly complete: boolean
}

const engine = {
  reset: resetBuildState,
  restore(state: SerializedBuildState) {
    const sheet: unknown = JSON.parse(state.sheet)
    const classes: unknown = JSON.parse(state.classes)
    const files: unknown = JSON.parse(state.files)
    const canonical: unknown = JSON.parse(state.canonical)
    importSheet(sheet)
    importClassMap(classes)
    importFileMap(files)
    importCanonicalMap(canonical)
  },
  capture(): SerializedBuildState {
    return {
      sheet: exportSheet(),
      classes: exportClassMap(),
      files: exportFileMap(),
      canonical: exportCanonicalMap(),
    }
  },
}

/** @internal Minted by the integration, never inferred from paths or caches. */
export function createWebpackGeneration(): BuildGeneration<SerializedBuildState> {
  return new BuildGeneration<SerializedBuildState>()
}

export class WebpackBuildScope {
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

  run<T>(action: () => T): T {
    return runBuildOperation(this.context, () =>
      this.binding.owner.run(engine, () => {
        this.configure()
        return action()
      }),
    )
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
    resetOwnedBuildState({ resetBuildState: engine.reset })
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
): T {
  const scope = compilerScope(compiler)
  return scope ? scope.run(action) : action()
}
