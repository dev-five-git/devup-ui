import { mkdir, rename, rm, writeFile } from 'node:fs/promises'
import { dirname } from 'node:path'

let temporaryFiles = 0

/** Write `content` so a reader sees the old file or the new one, never half. */
export async function writeFileAtomically(
  path: string,
  content: string,
): Promise<void> {
  const temporary = `${path}.${process.pid}.${++temporaryFiles}.tmp`
  await mkdir(dirname(path), { recursive: true })
  try {
    await writeFile(temporary, content, 'utf-8')
    await rename(temporary, path)
  } catch (error) {
    await rm(temporary, { force: true })
    throw error
  }
}

export type WriteOne = (
  path: string,
  content: string,
  encoding?: 'utf-8',
) => Promise<void>

export interface StateWriter {
  /**
   * Write `content` to `path` after every earlier write to it. A write that a
   * newer one for the same path overtakes before it starts is dropped, so the
   * file never ends up holding an older snapshot than one already handed in.
   */
  write(path: string, content: string, encoding?: 'utf-8'): Promise<void>
}

/**
 * Persist engine snapshots from concurrent transforms. Each call hands in the
 * state as it was when the call was made, and the last call made decides what
 * the file holds, whichever write the operating system finishes last.
 */
export function createStateWriter(
  writeOne: WriteOne = (path, content) => writeFileAtomically(path, content),
): StateWriter {
  const revisions = new Map<string, number>()
  const tails = new Map<string, Promise<void>>()
  return {
    write(path, content, encoding) {
      const revision = (revisions.get(path) ?? 0) + 1
      revisions.set(path, revision)
      const start = async () => {
        if (revisions.get(path) !== revision) return
        await (encoding
          ? writeOne(path, content, encoding)
          : writeOne(path, content))
      }
      const previous = tails.get(path)
      // With nothing in flight the write starts at once
      const run = previous
        ? previous.catch(() => undefined).then(start)
        : start()
      tails.set(path, run)
      const done = () => {
        if (tails.get(path) === run) tails.delete(path)
      }
      run.then(done, done)
      return run
    },
  }
}
