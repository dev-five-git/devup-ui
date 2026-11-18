import { createHash } from 'node:crypto'
import {
  readFileSync,
  rename,
  renameSync,
  rmSync,
  writeFile,
  writeFileSync,
} from 'node:fs'
import { dirname, join } from 'node:path'

/** The sheet, class map and file map files, in that order. */
export type SessionStateFiles = readonly [string, string, string]

/** The content of each of the `SessionStateFiles`, in the same order. */
export type SessionStateContents = readonly [string, string, string]

/**
 * Written after the three state files, with the hash of each. A restore that
 * finds a file whose hash differs reads a mix of two writes (a write that
 * stopped part way, or another process writing meanwhile) and starts fresh.
 */
export const SESSION_STATE_MANIFEST = 'stateManifest.json'

const MANIFEST_VERSION = 1

interface SessionStateManifest {
  version: number
  hashes: string[]
}

export function sessionStateManifestFile([
  sheetFile,
]: SessionStateFiles): string {
  return join(dirname(sheetFile), SESSION_STATE_MANIFEST)
}

function hash(content: string): string {
  return createHash('sha256').update(content).digest('hex')
}

function manifestFor(contents: SessionStateContents): string {
  return JSON.stringify({
    version: MANIFEST_VERSION,
    hashes: contents.map(hash),
  } satisfies SessionStateManifest)
}

// Windows refuses to replace a file another process holds open for a moment
// (an editor, a virus scanner, a loader reading the state).
const RETRY_CODES = new Set(['EPERM', 'EACCES', 'EBUSY'])
const RENAME_ATTEMPTS = 10
const RENAME_DELAY_MS = 10

function retryable(error: unknown, attempt: number): boolean {
  return (
    attempt < RENAME_ATTEMPTS &&
    RETRY_CODES.has((error as NodeJS.ErrnoException).code ?? '')
  )
}

let tempSeq = 0

/** A file next to `path`, so renaming it over `path` stays on one volume. */
function tempPath(path: string): string {
  tempSeq += 1
  return `${path}.${process.pid}.${tempSeq}.tmp`
}

function removeTemp(temp: string): void {
  rmSync(temp, { force: true })
}

function renameWithRetry(
  temp: string,
  path: string,
  attempt: number,
  done: (error: NodeJS.ErrnoException | null) => void,
): void {
  rename(temp, path, (error) => {
    if (error && retryable(error, attempt)) {
      setTimeout(
        () => renameWithRetry(temp, path, attempt + 1, done),
        RENAME_DELAY_MS * attempt,
      )
      return
    }
    done(error)
  })
}

/**
 * Write `content` to a temporary file and rename it over `path`, so a reader
 * (or a crash) never sees `path` half written.
 */
export function writeAtomic(path: string, content: string): Promise<void> {
  const temp = tempPath(path)
  return new Promise((resolve, reject) => {
    writeFile(temp, content, 'utf-8', (writeError) => {
      if (writeError) {
        removeTemp(temp)
        reject(writeError)
        return
      }
      renameWithRetry(temp, path, 1, (renameError) => {
        if (renameError) {
          removeTemp(temp)
          reject(renameError)
        } else resolve()
      })
    })
  })
}

function sleepSync(ms: number): void {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms)
}

/** `writeAtomic`, synchronously: for `process.on('exit')` and setup. */
export function writeAtomicSync(path: string, content: string): void {
  const temp = tempPath(path)
  try {
    writeFileSync(temp, content, 'utf-8')
    for (let attempt = 1; ; attempt++) {
      try {
        renameSync(temp, path)
        return
      } catch (error) {
        if (!retryable(error, attempt)) throw error
        sleepSync(RENAME_DELAY_MS * attempt)
      }
    }
  } catch (error) {
    removeTemp(temp)
    throw error
  }
}

/** Write the three state files, then the manifest that vouches for them. */
export async function writeSessionState(
  files: SessionStateFiles,
  contents: SessionStateContents,
): Promise<void> {
  await Promise.all(
    files.map((file, index) => writeAtomic(file, contents[index])),
  )
  await writeAtomic(sessionStateManifestFile(files), manifestFor(contents))
}

/** `writeSessionState`, synchronously. */
export function writeSessionStateSync(
  files: SessionStateFiles,
  contents: SessionStateContents,
): void {
  files.forEach((file, index) => writeAtomicSync(file, contents[index]))
  writeAtomicSync(sessionStateManifestFile(files), manifestFor(contents))
}

function manifestMatches(
  manifest: string,
  contents: SessionStateContents,
): boolean {
  try {
    const parsed = JSON.parse(manifest) as Partial<SessionStateManifest> | null
    return (
      parsed?.version === MANIFEST_VERSION &&
      Array.isArray(parsed.hashes) &&
      contents.every(
        (content, index) => parsed.hashes?.[index] === hash(content),
      )
    )
  } catch {
    return false
  }
}

/**
 * Read the state an earlier session wrote.
 *
 * - `undefined`: there is none to restore (first run, or a file is missing or
 *   unreadable).
 * - `null`: the files are not one write's (see `SESSION_STATE_MANIFEST`).
 * - otherwise the parsed sheet, class map and file map. State an earlier
 *   release wrote has no manifest and is read as before.
 */
export function readSessionState(
  files: SessionStateFiles,
): [unknown, unknown, unknown] | null | undefined {
  let contents: SessionStateContents
  try {
    contents = files.map((file) => readFileSync(file, 'utf-8')) as [
      string,
      string,
      string,
    ]
  } catch {
    return undefined
  }
  let manifest: string | undefined
  try {
    manifest = readFileSync(sessionStateManifestFile(files), 'utf-8')
  } catch {
    // Written by a release without manifests
  }
  if (manifest !== undefined && !manifestMatches(manifest, contents))
    return null
  try {
    return contents.map((content) => JSON.parse(content)) as [
      unknown,
      unknown,
      unknown,
    ]
  } catch {
    return undefined
  }
}
