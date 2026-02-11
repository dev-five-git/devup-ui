import type { ImportAliases } from './types'

/**
 * Extract file number from a devup-ui CSS filename.
 *
 * Handles both standard filenames (devup-ui-5.css) and query parameter
 * format (devup-ui.css?fileNum=79) used by Turbopack.
 *
 * @param filename - CSS filename or path to parse
 * @returns The file number, or null for the base devup-ui.css file
 */
export function getFileNumByFilename(filename: string): number | null {
  // Handle query parameter format: devup-ui.css?fileNum=79
  // Turbopack may embed query params in resourcePath
  const queryMatch = filename.match(/[?&]fileNum=(\d+)/)
  if (queryMatch) return parseInt(queryMatch[1])
  if (filename.endsWith('devup-ui.css')) return null
  return parseInt(filename.split('devup-ui-')[1].split('.')[0])
}

/**
 * Create a regex that excludes node_modules paths, except for @devup-ui
 * and any additional included packages.
 *
 * @param include - Additional package names to include (not exclude)
 * @param extraExcludes - Optional extra regex pattern to OR with the node_modules exclusion
 * @returns A RegExp for use in bundler exclude/condition rules
 */
export function createNodeModulesExcludeRegex(
  include: string[],
  extraExcludes?: string,
): RegExp {
  const base = `node_modules(?!.*(${['@devup-ui', ...include]
    .join('|')
    .replaceAll('/', '[\\/\\\\_]')})([\\/\\\\.]|$))`
  return new RegExp(extraExcludes ? `(${base})|(${extraExcludes})` : base)
}

/**
 * Common plugin options shared across all devup-ui build plugins.
 */
export interface DevupUIBasePluginOptions {
  package: string
  cssDir: string
  devupFile: string
  distDir: string
  debug: boolean
  include: string[]
  singleCss: boolean
  prefix?: string
  importAliases?: ImportAliases
}
