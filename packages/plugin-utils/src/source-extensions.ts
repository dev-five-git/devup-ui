import { extname } from 'node:path'

/**
 * The extensions of the JavaScript and TypeScript modules the extractor
 * compiles, ESM (`.mjs`, `.mts`) and CommonJS (`.cjs`, `.cts`) ones included.
 * Every plugin filter, the import graph and the module resolver take files
 * by this one list, so a file the graph reaches is also one the bundler
 * transforms.
 */
export const SOURCE_EXTENSIONS = [
  '.ts',
  '.tsx',
  '.mts',
  '.cts',
  '.js',
  '.jsx',
  '.mjs',
  '.cjs',
] as const

/** Whether a path ends in one of {@link SOURCE_EXTENSIONS}. */
export const SOURCE_FILE_REGEX = /\.(?:[mc]?[jt]s|[jt]sx)$/

/**
 * The TypeScript files an import of a JavaScript path can name, in the order
 * TypeScript and bundlers try them: ESM TypeScript is imported as `./a.js`,
 * `./a.mjs` or `./a.cjs` while the file on disk is `a.ts`, `a.mts` or `a.cts`.
 */
const TYPESCRIPT_FOR_JAVASCRIPT: Readonly<Record<string, readonly string[]>> = {
  '.js': ['.ts', '.tsx'],
  '.jsx': ['.tsx'],
  '.mjs': ['.mts'],
  '.cjs': ['.cts'],
}

/**
 * The files an import of `path` can load: `path` itself, then the TypeScript
 * sources a JavaScript extension stands for.
 */
export function sourceFileCandidates(path: string): string[] {
  const extension = extname(path)
  const base = path.slice(0, path.length - extension.length)
  return [
    path,
    ...(TYPESCRIPT_FOR_JAVASCRIPT[extension] ?? []).map(
      (typescript) => `${base}${typescript}`,
    ),
  ]
}
