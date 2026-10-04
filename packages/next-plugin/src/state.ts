import { randomUUID } from 'node:crypto'
import {
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { dirname } from 'node:path'

import { writeFileAtomically } from '@devup-ui/plugin-utils'

import type { DevupWasm } from './wasm'

export interface JsonObject {
  readonly [key: string]: unknown
}

/** One source the engine accepted, with what is needed to replay it. */
export interface CoordinatorInput {
  readonly filename: string
  readonly resourcePath: string
  readonly source: string
  /** Files the extraction read through the module resolver (root-relative) */
  readonly dependencies: readonly string[]
  /** Content hash of each dependency (absolute path) when it was accepted */
  readonly stamps: Readonly<Record<string, string>>
  /** Content hash of the file on disk when accepted, '' when it had none */
  readonly backing: string
}

/** Everything one coordinator needs to resume, committed as a single file. */
export interface CoordinatorSnapshot {
  readonly version: 1
  readonly optionsKey: string
  readonly project: string
  readonly revision: number
  readonly sheet: JsonObject
  readonly classMap: JsonObject
  readonly fileMap: JsonObject
  readonly inputs: readonly CoordinatorInput[]
}

export interface AllocatorState {
  readonly classMap: JsonObject
  readonly fileMap: JsonObject
}

export class CoordinatorStateError extends Error {
  constructor(stateFile: string, reason: string, cause?: unknown) {
    super(
      `${stateFile}:1:1: devup-ui coordinator checkpoint cannot use \`snapshot\` at build time: ${reason}. Fix: delete this file; the next dev/build run rebuilds the CSS state from the sources.`,
      { cause },
    )
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === 'string')
}

function isInput(value: unknown): boolean {
  return (
    isRecord(value) &&
    typeof value.filename === 'string' &&
    typeof value.resourcePath === 'string' &&
    typeof value.source === 'string' &&
    typeof value.backing === 'string' &&
    isStringArray(value.dependencies) &&
    isRecord(value.stamps) &&
    Object.values(value.stamps).every((stamp) => typeof stamp === 'string')
  )
}

const checks: readonly (readonly [string, (value: JsonObject) => boolean])[] = [
  ['version 1', (value) => value.version === 1],
  ['optionsKey string', (value) => typeof value.optionsKey === 'string'],
  ['project string', (value) => typeof value.project === 'string'],
  [
    'non-negative integer revision',
    (value) =>
      Number.isSafeInteger(value.revision) && Number(value.revision) >= 0,
  ],
  ['sheet object', (value) => isRecord(value.sheet)],
  ['classMap object', (value) => isRecord(value.classMap)],
  ['fileMap object', (value) => isRecord(value.fileMap)],
  [
    'inputs array of {filename, resourcePath, source, dependencies, stamps, backing}',
    (value) => Array.isArray(value.inputs) && value.inputs.every(isInput),
  ],
]

function assertSnapshot(
  stateFile: string,
  data: unknown,
): asserts data is CoordinatorSnapshot {
  const expected = isRecord(data)
    ? checks.find(([, valid]) => !valid(data))?.[0]
    : 'an object'
  if (expected !== undefined) {
    throw new CoordinatorStateError(
      stateFile,
      `is corrupt: expected ${expected}`,
    )
  }
}

function parseCheckpoint(stateFile: string): CoordinatorSnapshot {
  let data: unknown
  try {
    data = JSON.parse(readFileSync(stateFile, 'utf-8'))
  } catch (cause) {
    throw new CoordinatorStateError(stateFile, 'is not readable JSON', cause)
  }
  assertSnapshot(stateFile, data)
  return data
}

/**
 * The checkpoint at `stateFile`, validated as a whole. A missing file or one
 * written for other options is a cold start (`undefined`); anything else that
 * is not a complete snapshot throws a located error.
 */
export function readCoordinatorState(
  stateFile: string,
  optionsKey: string,
): CoordinatorSnapshot | undefined {
  if (!existsSync(stateFile)) return undefined
  const snapshot = parseCheckpoint(stateFile)
  return snapshot.optionsKey === optionsKey ? snapshot : undefined
}

/** The names and numbers handed out so far; kept across rebuilds. */
export function importAllocatorState(
  wasm: DevupWasm,
  allocator: AllocatorState,
): void {
  wasm.importClassMap(allocator.classMap)
  wasm.importFileMap(allocator.fileMap)
}

export function exportAllocatorState(wasm: DevupWasm): AllocatorState {
  return {
    classMap: JSON.parse(wasm.exportClassMap()),
    fileMap: JSON.parse(wasm.exportFileMap()),
  }
}

/**
 * Load every part of a snapshot into an engine nobody is using yet. A failure
 * part-way leaves that engine half restored, so the caller must discard it.
 */
export function restoreCoordinatorState(
  wasm: DevupWasm,
  snapshot: CoordinatorSnapshot,
): void {
  wasm.importSheet(snapshot.sheet)
  importAllocatorState(wasm, snapshot)
}

export interface CaptureRequest {
  readonly wasm: DevupWasm
  readonly optionsKey: string
  readonly project: string
  readonly revision: number
  readonly inputs: readonly CoordinatorInput[]
}

/** Export the engine's sheet and maps in one synchronous step. */
export function captureCoordinatorState(
  request: CaptureRequest,
): CoordinatorSnapshot {
  const { wasm, ...rest } = request
  return {
    version: 1,
    ...rest,
    sheet: JSON.parse(wasm.exportSheet()),
    ...exportAllocatorState(wasm),
  }
}

export function writeCoordinatorState(
  stateFile: string,
  snapshot: CoordinatorSnapshot,
): Promise<void> {
  return writeFileAtomically(stateFile, JSON.stringify(snapshot))
}

/** Synchronous variant for callers that cannot await (exit handlers). */
export function writeCoordinatorStateSync(
  stateFile: string,
  snapshot: CoordinatorSnapshot,
): void {
  const temporary = `${stateFile}.${process.pid}.${randomUUID()}.tmp`
  mkdirSync(dirname(stateFile), { recursive: true })
  try {
    writeFileSync(temporary, JSON.stringify(snapshot), 'utf-8')
    renameSync(temporary, stateFile)
  } catch (error) {
    rmSync(temporary, { force: true })
    throw error
  }
}

export interface SnapshotCommitter {
  /**
   * Persist what `capture` returns. Calls made before a write starts share it
   * and settle together, and `capture` runs only when the write starts, so the
   * newest capture wins and every caller's state is in the file it waited for.
   */
  commit(capture: () => CoordinatorSnapshot): Promise<void>
  /** Resolves once the latest complete capture is durable; rejects a failed write. */
  drain(): Promise<void>
}

interface Batch {
  capture: () => CoordinatorSnapshot
  readonly waiters: { resolve(): void; reject(error: unknown): void }[]
}

export function createSnapshotCommitter(
  write: (snapshot: CoordinatorSnapshot) => Promise<void>,
): SnapshotCommitter {
  let pending: Batch | undefined
  let running: Promise<void> | undefined
  let failure: { readonly error: unknown } | undefined
  async function run(): Promise<void> {
    // Yield first so `running` is assigned before this loop can finish
    await Promise.resolve()
    while (pending) {
      const batch = pending
      pending = undefined
      try {
        await write(batch.capture())
        failure = undefined
        for (const waiter of batch.waiters) waiter.resolve()
      } catch (error) {
        failure = { error }
        for (const waiter of batch.waiters) waiter.reject(error)
      }
    }
    running = undefined
  }
  return {
    commit(capture) {
      return new Promise((resolve, reject) => {
        pending ??= { capture, waiters: [] }
        pending.capture = capture
        pending.waiters.push({ resolve, reject })
        running ??= run()
      })
    },
    async drain() {
      while (running) await running
      if (failure !== undefined) throw failure.error
    },
  }
}
