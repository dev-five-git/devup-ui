import { readFileSync } from 'node:fs'
import { dirname, relative, resolve } from 'node:path'

import { locatedError } from './build-error'
import { takeExtractOutput } from './coordinator-engine'
import type { PrewarmedOutput } from './coordinator-options'
import { elapsedMs, profileStart, reportProfile } from './profile'
import type { AppContext } from './session'
import type { DevupWasm } from './wasm'

export interface PrewarmResult {
  /** The files that extracted, in extraction order */
  readonly files: string[]
  readonly outputs: Map<string, PrewarmedOutput>
}

interface RunPrewarmFields {
  context: AppContext
  engine: DevupWasm
  /** cwd-relative POSIX names, already in extraction order */
  files: readonly string[]
  /** Time spent choosing `files`, for the profile */
  collectMs: number | undefined
}

function extractOne(
  { context, engine }: Pick<RunPrewarmFields, 'context' | 'engine'>,
  filename: string,
): PrewarmedOutput {
  const resourcePath = resolve(context.root, filename)
  const relCssDir = `./${relative(dirname(resourcePath), context.cssDir).replaceAll('\\', '/')}`
  const source = readFileSync(resourcePath, 'utf-8')
  const extract = context.sourceMap
    ? engine.codeExtract
    : engine.codeExtractWithoutSourceMap
  const output = takeExtractOutput(
    extract(
      filename,
      source,
      context.libPackage,
      relCssDir,
      context.singleCss,
      false,
      true,
      context.importAliases,
    ),
  )
  return {
    code: output.code,
    cssFile: output.cssFile,
    map: output.map,
    source,
    updatedBaseStyle: output.updatedBaseStyle,
    dependencies: output.dependencies,
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
      if (fields.context.phase === 'production') throw error
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
  return { files, outputs }
}
