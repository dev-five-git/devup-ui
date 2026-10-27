import { isRecord } from './jsonc'
import { matchesTypeScriptVersion } from './typescript-version-range'

interface TargetPolicy {
  readonly purpose: 'module' | 'tsconfig-extends'
  readonly conditions: readonly string[]
  readonly load: (target: string) => string | undefined
}

export function conditionTarget(
  value: unknown,
  policy: TargetPolicy,
): string | null | undefined {
  if (typeof value === 'string') return policy.load(value)
  if (value === null) return null
  if (Array.isArray(value)) {
    for (const entry of value) {
      const target = conditionTarget(entry, policy)
      if (target !== undefined) return target
    }
    return undefined
  }
  if (!isRecord(value)) return undefined
  for (const [condition, branch] of Object.entries(value)) {
    if (
      condition !== 'default' &&
      !policy.conditions.includes(condition) &&
      !(
        policy.purpose === 'tsconfig-extends' &&
        condition.startsWith('types@') &&
        matchesTypeScriptVersion(condition.slice(6))
      )
    )
      continue
    const target = conditionTarget(branch, policy)
    if (target !== undefined) return target
  }
  return undefined
}

export function exportsTarget(
  exports: unknown,
  subpath: string,
  conditions: readonly string[],
): string | null | undefined {
  const subpaths = isRecord(exports)
    ? Object.keys(exports).filter((key) => key.startsWith('.'))
    : []
  const policy: TargetPolicy = {
    purpose: 'module',
    conditions,
    load: (target) => target,
  }
  if (subpaths.length === 0)
    return subpath === '.' ? conditionTarget(exports, policy) : undefined
  if (!isRecord(exports)) return undefined
  if (subpath in exports) return conditionTarget(exports[subpath], policy)
  for (const key of subpaths) {
    const [prefix, suffix = ''] = key.split('*')
    if (
      key.includes('*') &&
      subpath.startsWith(prefix) &&
      subpath.endsWith(suffix) &&
      subpath.length >= prefix.length + suffix.length
    ) {
      const matched = subpath.slice(
        prefix.length,
        subpath.length - suffix.length,
      )
      return conditionTarget(exports[key], policy)?.replaceAll('*', matched)
    }
  }
  return undefined
}

export function matchConfigTarget(
  table: unknown,
  request: string,
):
  | {
      readonly target: unknown
      readonly subpath: string
      readonly pattern: boolean
    }
  | undefined {
  if (!isRecord(table)) return undefined
  if (
    !request.endsWith('/') &&
    !request.includes('*') &&
    Object.hasOwn(table, request)
  )
    return { target: table[request], subpath: '', pattern: false }
  const keys = Object.keys(table).filter(
    (key) =>
      key.endsWith('/') ||
      (key.includes('*') && key.indexOf('*') === key.lastIndexOf('*')),
  )
  keys.sort((a, b) => {
    const ai = a.indexOf('*')
    const bi = b.indexOf('*')
    const length =
      (bi === -1 ? b.length : bi + 1) - (ai === -1 ? a.length : ai + 1)
    return length || (ai === -1 ? 1 : bi === -1 ? -1 : b.length - a.length)
  })
  for (const key of keys) {
    const star = key.indexOf('*')
    if (
      star !== -1 &&
      request.startsWith(key.slice(0, star)) &&
      request.endsWith(key.slice(star + 1))
    )
      return {
        target: table[key],
        subpath: request.substring(
          star,
          request.length - (key.length - star - 1),
        ),
        pattern: true,
      }
    if (request.startsWith(key))
      return {
        target: table[key],
        subpath: request.slice(key.length),
        pattern: false,
      }
  }
  return undefined
}
