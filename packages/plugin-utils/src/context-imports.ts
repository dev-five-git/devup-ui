import { readdirSync } from 'node:fs'
import { dirname, join, relative, resolve } from 'node:path'

/**
 * The files a module loads through a bundler's directory import, when its
 * arguments are written out: webpack's `require.context(dir, recursive,
 * /regex/)` and Vite's `import.meta.glob('pattern' | ['pattern', ...])`.
 * Without them, the start-of-build scan would never reach those files.
 */
export function contextImports(
  filename: string,
  source: string,
  root: string,
): string[] {
  const directory = dirname(filename)
  const files = new Set<string>()
  for (const match of source.matchAll(REQUIRE_CONTEXT)) {
    const [, , request = '', recursive, pattern, flags = ''] = match
    const base = resolve(directory, request)
    const test = pattern === undefined ? /^\.\/.*$/ : regex(pattern, flags)
    for (const file of listFiles(base, recursive !== 'false')) {
      const path = `./${relative(base, file).replaceAll('\\', '/')}`
      if (test?.test(path)) files.add(file)
    }
  }
  for (const match of source.matchAll(IMPORT_META_GLOB)) {
    for (const pattern of globPatterns(match[1] ?? '')) {
      const base = pattern.startsWith('/')
        ? resolve(root, `.${pattern}`)
        : resolve(directory, pattern)
      const start = staticPrefix(base)
      const test = globRegex(base.replaceAll('\\', '/'))
      for (const file of listFiles(start, true))
        if (test.test(file.replaceAll('\\', '/'))) files.add(file)
    }
  }
  return [...files].sort()
}

const REQUIRE_CONTEXT =
  /\brequire\.context\(\s*(['"])([^'"]+)\1\s*(?:,\s*(true|false)\s*)?(?:,\s*\/((?:\\.|[^/\\\n])+)\/([a-z]*)\s*)?(?:,\s*['"][^'"]*['"]\s*)?\)/g

const IMPORT_META_GLOB =
  /\bimport\.meta\.glob(?:Eager)?\s*(?:<[^>()]*>)?\(\s*((?:'[^']*'|"[^"]*")|\[\s*(?:(?:'[^']*'|"[^"]*")\s*,?\s*)*\])/g

function regex(pattern: string, flags: string): RegExp | undefined {
  try {
    return new RegExp(pattern, flags.replaceAll('g', ''))
  } catch {
    return undefined
  }
}

/** The positive patterns of a glob argument, a string or an array of them */
function globPatterns(argument: string): string[] {
  return [...argument.matchAll(/'([^']*)'|"([^"]*)"/g)]
    .map((match) => match[1] ?? match[2] ?? '')
    .filter((pattern) => pattern && !pattern.startsWith('!'))
}

const GLOB_CHARS = /[*?{[]/

/** The deepest directory of `pattern` before its first wildcard */
function staticPrefix(pattern: string): string {
  const index = pattern.search(GLOB_CHARS)
  return index < 0 ? dirname(pattern) : dirname(pattern.slice(0, index + 1))
}

/** A regular expression matching the paths `glob` matches */
function globRegex(glob: string): RegExp {
  let source = ''
  for (let index = 0; index < glob.length; index++) {
    const char = glob[index] ?? ''
    if (glob.startsWith('**/', index)) {
      source += '(?:.*/)?'
      index += 2
    } else if (glob.startsWith('**', index)) {
      source += '.*'
      index += 1
    } else if (char === '*') source += '[^/]*'
    else if (char === '?') source += '[^/]'
    else if (char === '{') source += '(?:'
    else if (char === '}') source += ')'
    else if (char === ',') source += '|'
    else source += char.replace(/[.+^$()|[\]\\]/g, '\\$&')
  }
  return new RegExp(`^${source}$`)
}

/** The files under `directory`, skipping dependencies and hidden folders */
function listFiles(directory: string, recursive: boolean): string[] {
  let entries
  try {
    entries = readdirSync(directory, { withFileTypes: true })
  } catch {
    return []
  }
  const files: string[] = []
  for (const entry of entries) {
    const path = join(directory, entry.name)
    if (entry.isFile()) files.push(path)
    else if (
      recursive &&
      entry.isDirectory() &&
      entry.name !== 'node_modules' &&
      !entry.name.startsWith('.')
    )
      files.push(...listFiles(path, true))
  }
  return files
}
