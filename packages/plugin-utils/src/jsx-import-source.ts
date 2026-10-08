import { existsSync, readFileSync, statSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, isAbsolute, join, resolve } from 'node:path'

interface TsConfig {
  extends?: unknown
  compilerOptions?: { jsxImportSource?: unknown }
  references?: unknown
}

/**
 * Where the comment starting at `index` of `text` ends, or `index` when none
 * starts there
 */
function skipComment(text: string, index: number): number {
  if (text.startsWith('//', index)) {
    const end = text.indexOf('\n', index)
    return end < 0 ? text.length : end
  }
  if (text.startsWith('/*', index)) {
    const end = text.indexOf('*/', index + 2)
    return end < 0 ? text.length : end + 2
  }
  return index
}

/**
 * The character after `index` of `text` that is neither space nor comment
 */
function nextToken(text: string, index: number): string | undefined {
  let at = index
  while (at < text.length) {
    const skipped = skipComment(text, at)
    if (skipped !== at) at = skipped
    else if (/\s/.test(text[at])) at++
    else return text[at]
  }
  return undefined
}

/**
 * JSON with the comments and trailing commas tsconfig allows, as JSON
 */
function stripJsonc(text: string): string {
  let json = ''
  let index = 0
  while (index < text.length) {
    const char = text[index]
    const skipped = skipComment(text, index)
    if (skipped !== index) {
      index = skipped
    } else if (char === '"') {
      const start = index
      index++
      while (index < text.length && text[index] !== '"') {
        index += text[index] === '\\' ? 2 : 1
      }
      index++
      json += text.slice(start, index)
    } else {
      const trailing =
        char === ',' && /[}\]]/.test(nextToken(text, index + 1) ?? '')
      if (!trailing) json += char
      index++
    }
  }
  return json
}

function readConfig(path: string): TsConfig | undefined {
  try {
    return JSON.parse(stripJsonc(readFileSync(path, 'utf-8'))) as TsConfig
  } catch {
    return undefined
  }
}

/**
 * The file an `extends` entry of the config at `from` names: a path, with
 * `.json` optional, or a package's config
 */
function resolveExtended(entry: string, from: string): string | undefined {
  if (entry.startsWith('.') || isAbsolute(entry)) {
    const path = resolve(dirname(from), entry)
    return [path, `${path}.json`].find((candidate) => existsSync(candidate))
  }
  const require = createRequire(from)
  for (const candidate of [entry, `${entry}/tsconfig.json`]) {
    try {
      return require.resolve(candidate)
    } catch {
      // Not a module path; the package may keep its config elsewhere
    }
  }
  return undefined
}

/**
 * The config a project reference names: a config file, or a directory
 * holding `tsconfig.json`
 */
function resolveReference(
  reference: unknown,
  from: string,
): string | undefined {
  const path =
    typeof reference === 'object' &&
    reference !== null &&
    'path' in reference &&
    typeof reference.path === 'string'
      ? resolve(dirname(from), reference.path)
      : undefined
  if (!path || !existsSync(path)) return undefined
  return statSync(path).isDirectory() ? join(path, 'tsconfig.json') : path
}

function jsxImportSourceOf(
  path: string,
  seen: Set<string>,
): string | undefined {
  if (seen.has(path)) return undefined
  seen.add(path)
  const config = readConfig(path)
  const own = config?.compilerOptions?.jsxImportSource
  if (typeof own === 'string') return own
  const extended = [config?.extends]
    .flat()
    .filter((entry): entry is string => typeof entry === 'string')
  // A later entry of `extends` overrides an earlier one
  for (const entry of extended.reverse()) {
    const parent = resolveExtended(entry, path)
    const source = parent && jsxImportSourceOf(parent, seen)
    if (source) return source
  }
  const references = Array.isArray(config?.references) ? config.references : []
  for (const reference of references) {
    const referenced = resolveReference(reference, path)
    const source = referenced && jsxImportSourceOf(referenced, seen)
    if (source) return source
  }
  return undefined
}

/**
 * The module the project builds JSX with, as its `tsconfig.json` (or, without
 * one, `jsconfig.json`) in `cwd` sets `compilerOptions.jsxImportSource`,
 * following `extends` and project references
 */
export function readJsxImportSource(
  cwd: string = process.cwd(),
): string | undefined {
  const config = ['tsconfig.json', 'jsconfig.json']
    .map((name) => join(cwd, name))
    .find((path) => existsSync(path))
  return config && jsxImportSourceOf(config, new Set())
}
