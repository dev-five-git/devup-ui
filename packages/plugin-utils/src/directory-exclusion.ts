import { posix, win32 } from 'node:path'

export function createDirectoryExclusion(
  exclude: readonly string[] = [],
  platform: string = process.platform,
): (directory: string) => boolean {
  const windows = platform === 'win32'
  const path = windows ? win32 : posix
  const normalize = (value: string) => {
    const normalized = path.normalize(
      windows ? value.replaceAll('/', '\\') : value,
    )
    const trimmed = normalized.replace(windows ? /\\+$/ : /\/+$/, '')
    return windows ? trimmed.toLowerCase() : trimmed
  }
  const names = new Set(
    exclude.filter((entry) => !path.isAbsolute(entry)).map(normalize),
  )
  const absolute = exclude
    .filter((entry) => path.isAbsolute(entry))
    .map(normalize)
  return (directory) => {
    const candidate = normalize(directory)
    if (candidate.split(windows ? '\\' : '/').some((part) => names.has(part)))
      return true
    return absolute.some(
      (root) => candidate === root || candidate.startsWith(root + path.sep),
    )
  }
}
