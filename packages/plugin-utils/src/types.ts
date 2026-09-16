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
 * Theme length definition
 * Each theme variant maps length token names to values
 * Values can be a single string/number or responsive array
 */
export type ThemeLength = Record<
  string,
  Record<string, string | number | (string | number | null)[]>
>

/**
 * Theme shadows definition
 * Each theme variant maps shadow token names to values
 * Values can be a single string or responsive array
 */
export type ThemeShadows = Record<
  string,
  Record<string, string | (string | null)[]>
>

/**
 * Theme configuration
 */
export interface DevupTheme {
  colors?: ThemeColors
  typography?: ThemeTypography
  length?: ThemeLength
  shadows?: ThemeShadows
}

/** Custom style prop aliases mapped to one or more CSS property names. */
export type CustomShorthands = Record<string, readonly string[]>

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
  '@emotion/react': true,
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

/**
 * Which `@devup-ui/react/compat` entry supplies the types for an aliased package.
 * Several specifiers can share one entry (both Emotion packages, for instance).
 */
const COMPAT_TYPE_ENTRIES: Record<string, string> = {
  '@emotion/react': 'emotion',
  '@emotion/styled': 'emotion',
  '@vanilla-extract/css': 'vanilla-extract',
  'styled-components': 'styled-components',
}

/**
 * Build the declaration file that makes aliased packages type-check without
 * being installed.
 *
 * Only enabled aliases are referenced: an ambient declaration wins over a real
 * installation, so a package the user opted out of must keep its own types.
 * StyleX is always included — the extractor recognises `@stylexjs/stylex`
 * directly rather than through the alias table.
 */
export function createCompatTypes(aliases: WasmImportAliases): string {
  const entries = new Set(['stylex'])
  for (const pkg of Object.keys(aliases)) {
    const entry = COMPAT_TYPE_ENTRIES[pkg]
    if (entry) entries.add(entry)
  }
  return [...entries]
    .sort()
    .map((entry) => `/// <reference types="@devup-ui/react/compat/${entry}" />`)
    .join('\n')
    .concat('\n')
}
