import { resolve } from 'node:path'

import { stampFile } from './coordinator-ledger'
import type { PrewarmedOutput } from './coordinator-options'
import type { PrewarmResult } from './prewarm-run'
import type { AppContext, AppSession } from './session'
import {
  captureCoordinatorState,
  type CoordinatorInput,
  importAllocatorState,
  readCoordinatorState,
  writeCoordinatorStateSync,
} from './state'
import type { DevupWasm } from './wasm'

export interface ResumedEngine {
  /** An engine nothing else uses, carrying the last session's names and numbers */
  readonly engine: DevupWasm
  readonly revision: number
}

interface ResumeFields {
  context: AppContext
  session: AppSession
  createEngine: () => DevupWasm
}

function warnColdStart(file: string, cause: unknown): void {
  const detail = cause instanceof Error ? cause.message : String(cause)
  console.warn(
    `[devup-ui] ${file}:1:1: devup-ui cannot use \`the previous session state\` at build time: ${detail}; needs a complete checkpoint. Starting cold: class names are numbered from the paths again instead of continuing the last session.`,
  )
}

/**
 * The engine a development session starts from. The names and numbers the last
 * session handed out are kept (class names do not jump between restarts); its
 * styles are not, because the current sources are replayed into a fresh sheet
 * so nothing a deleted or no longer reachable file produced survives.
 *
 * A checkpoint that is corrupt, or that fails to import part-way, is never
 * used: the engine it touched is dropped and a fresh one serves the session.
 * Production always starts fresh.
 */
export function resumeEngine({
  context,
  session,
  createEngine,
}: ResumeFields): ResumedEngine {
  const engine = createEngine()
  if (!context.watch) return { engine, revision: 0 }
  let revision = 0
  try {
    const checkpoint = readCoordinatorState(session.stateFile, context.appKey)
    if (checkpoint === undefined) return { engine, revision }
    revision = checkpoint.revision
    importAllocatorState(engine, checkpoint)
    return { engine, revision }
  } catch (cause) {
    warnColdStart(session.stateFile, cause)
    return { engine: createEngine(), revision }
  }
}

function toInput(
  context: AppContext,
  filename: string,
  output: PrewarmedOutput,
): CoordinatorInput {
  const resourcePath = resolve(context.root, filename)
  const dependencies = output.dependencies ?? []
  return {
    filename,
    resourcePath,
    source: output.source,
    dependencies,
    stamps: Object.fromEntries(
      dependencies.map((dependency) => {
        const path = resolve(context.root, dependency)
        return [path, stampFile(path)]
      }),
    ),
    backing: stampFile(resourcePath),
  }
}

interface PersistFields {
  context: AppContext
  session: AppSession
  engine: DevupWasm
  revision: number
  prewarm: PrewarmResult
}

/**
 * Commit the state after the prewarm, as one atomic file, before the
 * coordinator publishes its endpoint: whoever resumes from it finds the names
 * and the inputs that produced them together.
 */
export function persistInitialState({
  context,
  session,
  engine,
  revision,
  prewarm,
}: PersistFields): void {
  writeCoordinatorStateSync(
    session.stateFile,
    captureCoordinatorState({
      wasm: engine,
      optionsKey: context.appKey,
      project: context.root,
      revision,
      inputs: [...prewarm.outputs].map(([filename, output]) =>
        toInput(context, filename, output),
      ),
    }),
  )
}
