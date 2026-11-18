import { resolve } from 'node:path'

import type { DevupEngine } from './compiler-types'
import type { StylexOptions } from './stylex-options'
import type { CustomShorthands, ImportAliases } from './types'

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

  // Match against the BASENAME only. Splitting the whole path on "devup-ui-"
  // breaks when an ancestor directory itself contains "devup-ui-" (e.g. a
  // folder named `next-devup-ui-collapse`, or a project path containing the
  // package name): `split('devup-ui-')[1]` then grabs the wrong segment and
  // yields NaN -> null, so the css-loader silently serves the base sheet
  // instead of the per-file bucket (dropping collapsed-bucket atoms in
  // webpack, which uses the `devup-ui-N.css` filename form).
  const base = pathOnly.split(/[\\/]/).pop() ?? pathOnly
  if (base === 'devup-ui.css') return null

  const match = base.match(/^devup-ui-(\d+)\.css$/)
  if (!match) return null
  const num = parseInt(match[1], 10)
  return Number.isNaN(num) ? null : num
}

/**
 * Create a regex that excludes node_modules paths, except for @devup-ui
 * and any additional included packages.
 *
 * The package a file belongs to is the directory right under its innermost
 * `node_modules` directory (pnpm and Bun stores included), and a name matches
 * only that whole directory: `foo.bar` does not match `fooXbar`, `pkg` does not
 * match `@scope/pkg` or `pkg-extra`, a name with regex characters such as
 * `foo+bar` matches itself, and a dependency nested in an included package's
 * own `node_modules` is not included with it.
 *
 * @param include - Additional package names to include (not exclude)
 * @param extraExcludes - Optional extra regex pattern to OR with the node_modules exclusion
 * @returns A RegExp for use in bundler exclude/condition rules
 */
export function createNodeModulesExcludeRegex(
  include: readonly string[],
  extraExcludes?: string,
): RegExp {
  const separator = '[\\/\\\\]'
  const names = ['@devup-ui', '@devup-editor', ...include]
    .map((name) =>
      name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&').replaceAll('/', '[\\/\\\\_]'),
    )
    .join('|')
  const base = `(?:^|${separator})node_modules${separator}(?!.*${separator}node_modules${separator})(?!(?:${names})(?:${separator}|$))`
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
  engine?: DevupEngine
  package: string
  cssDir: string
  devupFile: string
  distDir: string
  debug: boolean
  include: string[]
  singleCss: boolean
  prefix?: string
  importAliases?: ImportAliases
  /** Build-time aliases that expand one style prop into multiple CSS properties. */
  shorthands?: CustomShorthands
  /** How code written against StyleX's API is read. */
  stylex?: StylexOptions
}

/**
 * The warning a plugin prints when an optional setup step fails and the build
 * goes on without it: the step, the root cause and what the build loses.
 */
export function setupWarning(
  step: string,
  error: unknown,
  impact: string,
): string {
  const cause = error instanceof Error ? error.message : String(error)
  return `[devup-ui] ${step} failed: ${cause}. ${impact}`
}

/**
 * Whether `error` reports that `dir` itself does not exist. A project without
 * that source root has nothing there to scan, so the step loses nothing.
 */
export function isMissingDirectory(error: unknown, dir: string): boolean {
  return (
    error instanceof Error &&
    'code' in error &&
    error.code === 'ENOENT' &&
    'path' in error &&
    typeof error.path === 'string' &&
    resolve(error.path) === resolve(dir)
  )
}
