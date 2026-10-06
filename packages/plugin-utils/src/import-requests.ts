import type { ImportRequestReference } from './import-scanner'

export type ImportRequestOutcome =
  | { readonly kind: 'resolved'; readonly path: string }
  | { readonly kind: 'external'; readonly request: string }
  | { readonly kind: 'ignored' }
  | { readonly kind: 'excluded'; readonly entry: string }
  | { readonly kind: 'unresolved' }
  | { readonly kind: 'error'; readonly error: unknown }

export interface ImportGraphRequest {
  readonly importer: string
  readonly request: string
  readonly specifier: string
  readonly kind: ImportRequestReference['requestKind']
  readonly position: ImportRequestReference['position']
  readonly source: 'source' | 'compiled'
  readonly map?: unknown
  readonly outcome: ImportRequestOutcome
}

const failures = new WeakMap<object, readonly ImportGraphRequest[]>()

export function importGraphFailureOf(
  error: unknown,
): readonly ImportGraphRequest[] | undefined {
  return (typeof error === 'object' && error !== null) ||
    typeof error === 'function'
    ? failures.get(error)
    : undefined
}

export function retainImportGraphFailure(
  error: unknown,
  request: ImportGraphRequest,
): void {
  if (
    (typeof error === 'object' && error !== null) ||
    typeof error === 'function'
  )
    failures.set(error, freezeImportRequests([request]))
}

export function freezeImportRequests(
  requests: ImportGraphRequest[],
): readonly ImportGraphRequest[] {
  requests.sort((a, b) =>
    a.importer < b.importer
      ? -1
      : a.importer > b.importer
        ? 1
        : a.position.offset - b.position.offset,
  )
  for (const request of requests) {
    Object.freeze(request.position)
    Object.freeze(request.outcome)
    Object.freeze(request)
  }
  return Object.freeze(requests)
}
