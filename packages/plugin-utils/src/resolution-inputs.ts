import { readFileSync, realpathSync } from 'node:fs'
import { resolve } from 'node:path'

export interface ResolutionInputs {
  readonly fileDependencies: readonly string[]
  readonly missingDependencies: readonly string[]
}

export type ResolutionInputObserver = (inputs: ResolutionInputs) => void

export function createResolutionInputs(initial?: ResolutionInputs) {
  const files = new Set(initial?.fileDependencies)
  const missing = new Set(initial?.missingDependencies)
  return {
    file(path: string) {
      files.add(resolve(path))
    },
    missing(path: string) {
      missing.add(resolve(path))
    },
    probe(path: string, exists: boolean) {
      if (!exists) missing.add(resolve(path))
    },
    snapshot(): ResolutionInputs {
      return Object.freeze({
        fileDependencies: Object.freeze([...files].sort()),
        missingDependencies: Object.freeze([...missing].sort()),
      })
    },
  }
}

export type ResolutionInputCollector = ReturnType<typeof createResolutionInputs>

export function recordResolutionFailure(
  path: string,
  error: unknown,
  inputs?: ResolutionInputCollector,
): void {
  const code: unknown =
    error instanceof Error
      ? Object.getOwnPropertyDescriptor(error, 'code')?.value
      : undefined
  if (code === 'ENOENT' || code === 'ENOTDIR') inputs?.missing(path)
  else inputs?.file(path)
}

export function realpathResolution(
  path: string,
  inputs?: ResolutionInputCollector,
): string {
  try {
    return realpathSync(path)
  } catch (error) {
    recordResolutionFailure(path, error, inputs)
    throw error
  }
}

export function readResolutionFile(
  path: string,
  inputs?: ResolutionInputCollector,
): string {
  try {
    const source = readFileSync(path, 'utf-8')
    inputs?.file(path)
    return source
  } catch (error) {
    recordResolutionFailure(path, error, inputs)
    throw error
  }
}
