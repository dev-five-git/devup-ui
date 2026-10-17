export type NonphysicalIdListOutcome =
  | { readonly kind: 'loaded'; readonly ids: readonly string[] }
  | { readonly kind: 'missing'; readonly ids: readonly [] }
  | { readonly kind: 'corrupt'; readonly ids: readonly [] }

export function parseNonphysicalIdList(
  serialized: string | undefined,
): NonphysicalIdListOutcome {
  if (serialized === undefined)
    return Object.freeze({ kind: 'missing', ids: Object.freeze<[]>([]) })
  let parsed: unknown
  try {
    parsed = JSON.parse(serialized)
  } catch (error) {
    if (error instanceof SyntaxError)
      return Object.freeze({ kind: 'corrupt', ids: Object.freeze<[]>([]) })
    throw error
  }
  if (!Array.isArray(parsed))
    return Object.freeze({ kind: 'corrupt', ids: Object.freeze<[]>([]) })
  const ids = new Set<string>()
  for (const element of parsed) {
    if (typeof element !== 'string' || element.length === 0)
      return Object.freeze({ kind: 'corrupt', ids: Object.freeze<[]>([]) })
    ids.add(element)
  }
  return Object.freeze({ kind: 'loaded', ids: Object.freeze([...ids].sort()) })
}

export function nextNonphysicalIdList(
  extractedNonphysical: readonly string[],
  knownNonphysicalScan: readonly string[],
): readonly string[] {
  const known = new Set(knownNonphysicalScan)
  const ids = [...new Set(extractedNonphysical)].filter((id) => !known.has(id))
  return Object.freeze(ids.sort())
}

export function serializeNonphysicalIdList(ids: readonly string[]): string {
  return JSON.stringify([...new Set(ids)].sort())
}
