import type { ImportAliases } from './types'

/**
 * Extract file number from a devup-ui CSS filename.
 *
 * Handles both standard filenames (devup-ui-5.css) and query parameter
 * format (devup-ui.css?fileNum=79) used by Turbopack.
 *
 * Next.js may append additional query parameters (e.g. `?dpl=DEPLOYMENT_ID`)
 * to module URLs when `assetPrefix` is set. Such queries must be stripped
 * before matching the base filename, otherwise the base CSS request would
 * be misidentified and the `@layer b` styles would be dropped from the
 * build output.
 *
 * @param filename - CSS filename or path to parse
 * @returns The file number, or null for the base devup-ui.css file
 */
export function getFileNumByFilename(filename: string): number | null {
  // Handle query parameter format: devup-ui.css?fileNum=79
  // Turbopack may embed query params in resourcePath
  const queryMatch = filename.match(/[?&]fileNum=(\d+)/)
  if (queryMatch) return parseInt(queryMatch[1], 10)

  // Strip query string before matching the filename pattern. Next.js can
  // append arbitrary queries (e.g. `?dpl=...`) when assetPrefix is set, and
  // those must not interfere with base CSS detection.
  const pathOnly = filename.split('?')[0]
  if (pathOnly.endsWith('devup-ui.css')) return null

  const numericPart = pathOnly.split('devup-ui-')[1]?.split('.')[0]
  if (numericPart === undefined) return null
  const num = parseInt(numericPart, 10)
  return Number.isNaN(num) ? null : num
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
  const base = `node_modules(?!.*(${['@devup-ui', '@devup-editor', ...include]
    .join('|')
    .replaceAll('/', '[\\/\\\\_]')})([\\/\\\\.]|$))`
  return new RegExp(extraExcludes ? `(${base})|(${extraExcludes})` : base)
}

export interface DevupThemeInterfaceNames {
  color: string
  typography: string
  length: string
  shadow: string
  theme: string
}

export const DEFAULT_THEME_INTERFACE_NAMES = {
  color: 'CustomColors',
  typography: 'DevupThemeTypography',
  length: 'CustomLength',
  shadow: 'CustomShadows',
  theme: 'DevupTheme',
} satisfies DevupThemeInterfaceNames

export function createThemeInterfaceArgs(
  packageName: string,
  names: DevupThemeInterfaceNames = DEFAULT_THEME_INTERFACE_NAMES,
): [string, string, string, string, string, string] {
  return [
    packageName,
    names.color,
    names.typography,
    names.length,
    names.shadow,
    names.theme,
  ]
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
