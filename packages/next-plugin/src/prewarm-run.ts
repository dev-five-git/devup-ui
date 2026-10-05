import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { locatedError } from './build-error'
import { extractInput } from './coordinator-engine'
import type { PrewarmedOutput } from './coordinator-options'
import { elapsedMs, profileStart, reportProfile } from './profile'
import type { AppContext } from './session'
import type { DevupWasm } from './wasm'

export interface PrewarmResult {
  /** The files that extracted, in extraction order */
  readonly files: string[]
  readonly outputs: Map<string, PrewarmedOutput>
  readonly preparedInputs?: ReadonlyMap<string, PreparedPrewarmInput>
}

export interface PreparedPrewarmInput {
  readonly code: string
  readonly map?: unknown
  readonly dependencies?: readonly string[]
}

export interface RunPrewarmFields {
  context: AppContext
  engine: DevupWasm
  /** cwd-relative POSIX names, already in extraction order */
  files: readonly string[]
  /** Time spent choosing `files`, for the profile */
  collectMs: number | undefined
  /** Complete pre-Devup inputs; supplying this makes preparation required in dev too. */
  readonly preparedInputs?: ReadonlyMap<string, PreparedPrewarmInput>
}

function extractOne(
  { context, engine, preparedInputs }: RunPrewarmFields,
  filename: string,
): PrewarmedOutput {
  const resourcePath = resolve(context.root, filename)
  const prepared = preparedInputs?.get(filename)
  if (preparedInputs && prepared === undefined) {
    throw new TypeError(`Required prepared source is missing: ${resourcePath}`)
  }
  const source = prepared?.code ?? readFileSync(resourcePath, 'utf-8')
  const output = extractInput(
    engine,
    {
      package: context.libPackage,
      cssDir: context.cssDir,
      singleCss: context.singleCss,
      sourceMap: context.sourceMap,
      importAliases: context.importAliases,
    },
    { filename, resourcePath, source },
  )
  return {
    code: output.code,
    cssFile: output.cssFile,
    map: output.map,
    source,
    updatedBaseStyle: output.updatedBaseStyle,
    dependencies: prepared
      ? [...(output.dependencies ?? []), ...(prepared.dependencies ?? [])]
      : output.dependencies,
  }
}

/**
 * Extract `files` in the given order, before the bundler asks for anything, so
 * class names and numbers follow the paths and not the order requests arrive
 * in. A file that cannot be extracted fails a production build with its
 * location; in development it is reported and skipped (the bundler hits the
 * same error when it compiles the file).
 */
export function runPrewarm(fields: RunPrewarmFields): PrewarmResult {
  const startedAt = profileStart()
  const outputs = new Map<string, PrewarmedOutput>()
  let extractMs = 0
  let sourceBytes = 0
  for (const filename of fields.files) {
    const extractStartedAt = performance.now()
    try {
      const output = extractOne(fields, filename)
      outputs.set(filename, output)
      sourceBytes += Buffer.byteLength(output.source)
    } catch (cause) {
      const error = locatedError({
        file: resolve(fields.context.root, filename),
        what: 'devup-ui prewarm',
        code: filename,
        needs: 'a readable source file the extractor can compile',
        cause,
      })
      if (fields.preparedInputs || fields.context.phase === 'production')
        throw error
      console.warn(
        `[devup-ui] ${error.message}. Skipped while prewarming; its styles are extracted when the bundler compiles it.`,
      )
    }
    extractMs += performance.now() - extractStartedAt
  }
  const files = [...outputs.keys()]
  reportProfile('next.prewarm', {
    collectMs: fields.collectMs,
    durationMs: elapsedMs(startedAt),
    extractMs: Number(extractMs.toFixed(2)),
    files: files.length,
    sourceBytes,
  })
  return {
    files,
    outputs,
    ...(fields.preparedInputs === undefined
      ? {}
      : { preparedInputs: fields.preparedInputs }),
  }
}
