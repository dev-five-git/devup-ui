import { dirname, relative } from 'node:path'

import type { ExtractRequest } from './coordinator-http'
import type { PrewarmedOutput } from './coordinator-options'
import {
  type AllocatorState,
  type CoordinatorInput,
  importAllocatorState,
} from './state'
import type { DevupWasm } from './wasm'

export interface ExtractOutputSnapshot extends Omit<PrewarmedOutput, 'source'> {
  css?: string
}

/** Copy every WASM-backed getter once, then release its Rust allocation. */
export function takeExtractOutput(
  output: ReturnType<DevupWasm['codeExtract']>,
): ExtractOutputSnapshot {
  try {
    return {
      code: output.code,
      css: output.css,
      cssFile: output.cssFile,
      map: output.map,
      updatedBaseStyle: output.updatedBaseStyle,
      dependencies: output.dependencies,
    }
  } finally {
    output.free()
  }
}

export interface ExtractResponse {
  code: string
  map?: string
  cssFile?: string
  updatedBaseStyle: boolean
  dependencies: string[]
}

/**
 * What the loader gets back. Per-file CSS imports become a query on the
 * placeholder Turbopack can resolve (devup-ui-79.css -> devup-ui.css?fileNum=79);
 * `watched` are extra files the loader must depend on (the theme chain).
 */
export function toExtractResponse(
  output: ExtractOutputSnapshot,
  singleCss: boolean,
  watched: readonly string[],
): ExtractResponse {
  return {
    code:
      !singleCss && output.code
        ? output.code.replace(/devup-ui-(\d+)\.css/g, 'devup-ui.css?fileNum=$1')
        : output.code,
    map: output.map,
    cssFile: output.cssFile,
    updatedBaseStyle: output.updatedBaseStyle,
    dependencies: [...(output.dependencies ?? []), ...watched],
  }
}

const LOCATED = /^.+:\d+:\d+: /

/**
 * An error that names the file (`file:1:1`), what cannot be built and the fix.
 * Engine errors that already carry a position are kept as they are.
 */
export function locatedError(
  file: string,
  what: string,
  reason: unknown,
  fix: string,
): Error {
  const detail = reason instanceof Error ? reason.message : String(reason)
  return LOCATED.test(detail)
    ? new Error(detail, { cause: reason })
    : new Error(
        `${file}:1:1: devup-ui coordinator cannot ${what}: ${detail}. Fix: ${fix}`,
        { cause: reason },
      )
}

export interface ExtractSettings {
  readonly package: string
  readonly cssDir: string
  readonly singleCss: boolean
  readonly sourceMap: boolean
  readonly importAliases: Record<string, string | null>
}

export function extractInput(
  wasm: DevupWasm,
  settings: ExtractSettings,
  input: Pick<CoordinatorInput, 'filename' | 'resourcePath' | 'source'>,
): ExtractOutputSnapshot {
  let relCssDir = relative(
    dirname(input.resourcePath),
    settings.cssDir,
  ).replaceAll('\\', '/')
  if (!relCssDir.startsWith('./')) relCssDir = `./${relCssDir}`
  const extract = settings.sourceMap
    ? wasm.codeExtract
    : wasm.codeExtractWithoutSourceMap
  return takeExtractOutput(
    extract(
      input.filename,
      input.source,
      settings.package,
      relCssDir,
      settings.singleCss,
      false,
      true,
      settings.importAliases,
    ),
  )
}

/** Extract a loader's request, failing with an error that names the file and the fix. */
export function extractRequest(
  wasm: DevupWasm,
  settings: ExtractSettings,
  {
    code,
    ...request
  }: Pick<ExtractRequest, 'filename' | 'resourcePath' | 'code'>,
): ExtractOutputSnapshot {
  try {
    return extractInput(wasm, settings, { ...request, source: code })
  } catch (error) {
    throw locatedError(
      request.filename,
      'extract styles',
      error,
      'fix the reported source, or remove the Devup UI usage that cannot be resolved at build time.',
    )
  }
}

export interface RebuildPlan {
  readonly createEngine: () => DevupWasm
  /** The engine in service; the new one must not be it. */
  readonly live: DevupWasm
  readonly configure: (wasm: DevupWasm) => void
  /** Names and numbers to carry over, so survivors keep theirs. */
  readonly allocator: AllocatorState
  readonly theme: object | undefined
  readonly settings: ExtractSettings
  readonly inputs: readonly CoordinatorInput[]
}

/**
 * A fresh engine holding exactly `inputs`: the allocator state first (so
 * deleted files leave their numbers behind as tombstones), then the plugin's
 * configuration and theme, then every input replayed in order.
 */
export function buildEngine(plan: RebuildPlan): DevupWasm {
  const fresh = plan.createEngine()
  if (fresh === plan.live) {
    throw new Error(
      'devup-ui coordinator cannot rebuild CSS state: createEngine returned the engine in service. Fix: createEngine must return a new, isolated engine on every call.',
    )
  }
  importAllocatorState(fresh, plan.allocator)
  plan.configure(fresh)
  if (plan.theme !== undefined) fresh.registerTheme(plan.theme)
  for (const input of plan.inputs) {
    try {
      extractInput(fresh, plan.settings, input)
    } catch (error) {
      throw locatedError(
        input.filename,
        'rebuild the CSS state after a source or theme change',
        error,
        'fix this file (or restore what it imports); the CSS is served from the live state until it builds again.',
      )
    }
  }
  return fresh
}
