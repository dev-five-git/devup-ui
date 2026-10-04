import { createHash } from 'node:crypto'
import { existsSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import type { ExtractOutputSnapshot } from './coordinator-engine'
import type { ExtractRequest } from './coordinator-http'
import type { CoordinatorInput } from './state'

function hash(content: string | Buffer): string {
  return createHash('sha1').update(content).digest('hex')
}

/** Content hash of a file, or '' when there is no such file. */
export function stampFile(path: string): string {
  return existsSync(path) ? hash(readFileSync(path)) : ''
}

/** The ledger entry for a source, stamping the files it read as of now. */
export function createInput(
  root: string,
  {
    code,
    ...request
  }: Pick<ExtractRequest, 'filename' | 'resourcePath' | 'code'>,
  dependencies: readonly string[],
): CoordinatorInput {
  return {
    ...request,
    source: code,
    dependencies,
    stamps: Object.fromEntries(
      dependencies.map((dependency) => {
        const path = resolve(root, dependency)
        return [path, stampFile(path)]
      }),
    ),
    backing: stampFile(request.resourcePath),
  }
}

/** The file the input came from was deleted or renamed (virtual inputs never are). */
export function isGone(input: CoordinatorInput): boolean {
  return input.backing !== '' && !existsSync(input.resourcePath)
}

/** The file is still what the checkpoint accepted. */
export function isCurrent(input: CoordinatorInput): boolean {
  return input.backing === '' || stampFile(input.resourcePath) === input.backing
}

export interface InputLedger {
  /** Record an accepted source together with the output it produced. */
  accept(input: CoordinatorInput, output: ExtractOutputSnapshot): void
  /**
   * The output of an earlier identical transform, if the engine still holds
   * exactly that source and nothing it read has changed since.
   */
  lookup(filename: string, source: string): ExtractOutputSnapshot | undefined
  /** Live inputs in replay (path) order. */
  list(): CoordinatorInput[]
  /** Swap the live inputs wholesale; cached outputs no longer apply. */
  replace(inputs: readonly CoordinatorInput[]): void
}

/**
 * What the engine currently holds, per file. The inputs are the live accepted
 * sources (needed to replay them); the outputs are a bounded cache of what
 * they produced, so identical transforms from Next's server and client graphs
 * do not run the engine twice.
 */
export function createInputLedger(maxOutputs: number): InputLedger {
  let inputs = new Map<string, CoordinatorInput>()
  const outputs = new Map<string, ExtractOutputSnapshot>()
  const keyOf = (filename: string, source: string) =>
    `${filename}\0${hash(source)}`
  return {
    accept(input, output) {
      inputs.set(input.filename, input)
      const key = keyOf(input.filename, input.source)
      outputs.delete(key)
      outputs.set(key, output)
      for (const oldest of outputs.keys()) {
        if (outputs.size <= maxOutputs) break
        outputs.delete(oldest)
      }
    },
    lookup(filename, source) {
      const input = inputs.get(filename)
      const fresh =
        input?.source === source &&
        Object.entries(input.stamps).every(
          ([path, stamp]) => stampFile(path) === stamp,
        )
      const key = keyOf(filename, source)
      const output = fresh ? outputs.get(key) : undefined
      if (output) {
        outputs.delete(key)
        outputs.set(key, output)
      }
      return output
    },
    list() {
      return [...inputs.values()].sort((a, b) =>
        a.filename < b.filename ? -1 : a.filename > b.filename ? 1 : 0,
      )
    },
    replace(next) {
      inputs = new Map(next.map((input) => [input.filename, input]))
      outputs.clear()
    },
  }
}
