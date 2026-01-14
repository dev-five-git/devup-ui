/**
 * Typography definition for a single breakpoint or non-responsive typography
 */
export interface Typography {
  fontFamily?: string
  fontStyle?: string
  fontWeight?: number | string
  fontSize?: string
  lineHeight?: number | string
  letterSpacing?: string
}

/**
 * Theme colors definition
 * Each theme variant (e.g., 'default', 'dark', 'light') maps color names to values
 */
export type ThemeColors = Record<string, Record<string, string>>

/**
 * Theme typography definition
 * Each typography name maps to either a single Typography or an array for responsive values
 */
export type ThemeTypography = Record<string, Typography | (Typography | null)[]>

/**
 * Theme configuration
 */
export interface DevupTheme {
  colors?: ThemeColors
  typography?: ThemeTypography
}

/**
 * Devup configuration file structure (devup.json)
 */
export interface DevupConfig {
  /**
   * Array of paths to extend from
   * Paths are resolved relative to the config file
   * First item is the base, subsequent items override in order
   * The current config is applied last (highest priority)
   */
  extends?: string[]

  /**
   * Theme configuration
   */
  theme?: DevupTheme
}

/**
 * Import alias configuration for redirecting imports from other CSS-in-JS libraries
 *
 * - `string`: default export → named export (e.g., `'styled'` transforms `import styled from 'pkg'` to `import { styled } from '@devup-ui/react'`)
 * - `true`: all named exports (1:1 mapping)
 * - `false`: disable this alias
 *
 * @example
 * ```ts
 * {
 *   '@emotion/styled': 'styled',      // default export → named 'styled'
 *   'styled-components': 'styled',    // default export → named 'styled'
 *   '@vanilla-extract/css': true,     // named exports (1:1)
 *   'some-lib': false                 // disable
 * }
 * ```
 */
export type ImportAliases = Record<string, string | true | false>

/**
 * Default import aliases for common CSS-in-JS libraries
 */
export const DEFAULT_IMPORT_ALIASES: ImportAliases = {
  '@emotion/styled': 'styled',
  'styled-components': 'styled',
  '@vanilla-extract/css': true,
}

/**
 * WASM-compatible import aliases format
 * - `string`: default export → named export
 * - `null`: named exports (1:1 mapping)
 */
export type WasmImportAliases = Record<string, string | null>

/**
 * Merge user import aliases with defaults and convert to WASM format
 *
 * @param userAliases - User-provided aliases (optional)
 * @returns WASM-compatible import aliases
 *
 * @example
 * ```ts
 * const aliases = mergeImportAliases({ '@emotion/styled': false })
 * // Returns: { 'styled-components': 'styled', '@vanilla-extract/css': null }
 * ```
 */
export function mergeImportAliases(
  userAliases?: ImportAliases,
): WasmImportAliases {
  const merged = { ...DEFAULT_IMPORT_ALIASES, ...userAliases }
  return Object.fromEntries(
    Object.entries(merged)
      .filter((entry): entry is [string, string | true] => entry[1] !== false)
      .map(([pkg, value]) => [pkg, value === true ? null : value]),
  ) as WasmImportAliases
}
