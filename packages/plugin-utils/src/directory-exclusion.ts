import { posix, win32 } from 'node:path'

export function createDirectoryExclusion(
  exclude: readonly string[] = [],
  platform: string = process.platform,
): ((directory: string) => boolean) & {
  readonly match: (directory: string) => string | undefined
} {
  const windows = platform === 'win32'
  const path = windows ? win32 : posix
  const normalize = (value: string) => {
    const normalized = path.normalize(
      windows ? value.replaceAll('/', '\\') : value,
    )
    const trimmed = normalized.replace(windows ? /\\+$/ : /\/+$/, '')
    return windows ? trimmed.toLowerCase() : trimmed
  }
  const names = new Map(
    exclude
      .filter((entry) => !path.isAbsolute(entry))
      .map((entry) => [normalize(entry), entry]),
  )
  const absolute = exclude
    .filter((entry) => path.isAbsolute(entry))
    .map((entry) => ({ normalized: normalize(entry), entry }))
  const match = (directory: string) => {
    const candidate = normalize(directory)
    for (const part of candidate.split(windows ? '\\' : '/')) {
      const entry = names.get(part)
      if (entry !== undefined) return entry
    }
    return absolute.find(
      ({ normalized }) =>
        candidate === normalized || candidate.startsWith(normalized + path.sep),
    )?.entry
  }
  return Object.assign((directory: string) => match(directory) !== undefined, {
    match,
  })
}
