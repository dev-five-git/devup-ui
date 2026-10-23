import { basename } from 'node:path'

import { ConfigLoadError } from './load-config'
import { PackageManifestShapeError } from './package-manifest'

const READ_FAILURES = new Set<string>([
  'ENOENT',
  'EACCES',
  'EPERM',
  'ENOTDIR',
  'EISDIR',
])

export function isReservationReadFailure(error: unknown): boolean {
  return (
    error instanceof Error &&
    'code' in error &&
    typeof error.code === 'string' &&
    READ_FAILURES.has(error.code)
  )
}

function manifestCause(error: unknown): unknown {
  return error instanceof ConfigLoadError &&
    basename(error.file) === 'package.json'
    ? error.cause
    : error
}

/** A package.json that is not valid JSON or not an object. */
export function isManifestShapeFailure(error: unknown): boolean {
  const cause = manifestCause(error)
  return (
    cause instanceof SyntaxError || cause instanceof PackageManifestShapeError
  )
}

export function reservationSource<T>(
  read: () => T,
  manifestBoundary = false,
): T | undefined {
  try {
    return read()
  } catch (error) {
    if (
      isReservationReadFailure(
        manifestBoundary ? manifestCause(error) : error,
      ) ||
      (manifestBoundary && isManifestShapeFailure(error))
    ) {
      return undefined
    }
    throw error
  }
}
