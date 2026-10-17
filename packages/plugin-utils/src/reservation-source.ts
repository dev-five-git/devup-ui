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

export function reservationSource<T>(
  read: () => T,
  manifestBoundary = false,
): T | undefined {
  try {
    return read()
  } catch (error) {
    if (
      isReservationReadFailure(error) ||
      (manifestBoundary &&
        (error instanceof SyntaxError ||
          error instanceof PackageManifestShapeError))
    ) {
      return undefined
    }
    throw error
  }
}
