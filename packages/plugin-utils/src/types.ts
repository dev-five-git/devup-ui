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
