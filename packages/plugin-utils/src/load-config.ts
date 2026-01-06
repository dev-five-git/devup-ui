import { existsSync, readFileSync } from 'node:fs'
import { readFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'

import type { DevupConfig } from './types'

/**
 * Check if a value is a plain object
 */
function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

/**
 * Deep merge two objects
 * Arrays are replaced, not merged
 * Objects are recursively merged
 * The second object (override) takes precedence
 */
export function deepMerge<T, U>(base: T, override: U): T {
  if (!isPlainObject(base) || !isPlainObject(override)) {
    return (override !== undefined ? override : base) as T
  }

  const result = { ...base } as T

  for (const key in override) {
    if (Object.prototype.hasOwnProperty.call(override, key)) {
      const baseValue = (base as Record<string, unknown>)[key]
      const overrideValue = (override as Record<string, unknown>)[key]

      if (isPlainObject(baseValue) && isPlainObject(overrideValue)) {
        // Recursively merge objects
        ;(result as Record<string, unknown>)[key] = deepMerge(
          baseValue as Record<string, unknown>,
          overrideValue as Record<string, unknown>,
        )
      } else if (overrideValue !== undefined) {
        // Override with the new value (including arrays)
        ;(result as Record<string, unknown>)[key] = overrideValue
      }
    }
  }

  return result
}

/**
 * Parse JSON content safely
 */
function parseConfig(content: string): DevupConfig {
  try {
    return JSON.parse(content) as DevupConfig
  } catch {
    return {}
  }
}

/**
 * Load and resolve a devup.json config file synchronously
 * Handles the extends field by loading and merging parent configs
 *
 * @param configPath - Path to the devup.json file
 * @returns Resolved configuration with all extends merged
 */
export function loadDevupConfigSync(configPath: string): DevupConfig {
  if (!existsSync(configPath)) {
    return {}
  }

  const content = readFileSync(configPath, 'utf-8')
  const config = parseConfig(content)

  // If no extends, return the config as-is
  if (!config.extends || config.extends.length === 0) {
    return config
  }

  const configDir = dirname(configPath)

  // Start with empty base and merge extends in order
  // First extend is the base, subsequent ones override
  let mergedConfig: DevupConfig = {}

  for (const extendPath of config.extends) {
    const resolvedPath = resolve(configDir, extendPath)
    const extendedConfig = loadDevupConfigSync(resolvedPath)
    mergedConfig = deepMerge(mergedConfig, extendedConfig)
  }

  // Finally, merge the current config (highest priority)
  // Remove extends from the result as it's already resolved
  const { extends: _, ...currentConfig } = config
  return deepMerge(mergedConfig, currentConfig)
}

/**
 * Load and resolve a devup.json config file asynchronously
 * Handles the extends field by loading and merging parent configs
 *
 * @param configPath - Path to the devup.json file
 * @returns Resolved configuration with all extends merged
 */
export async function loadDevupConfig(
  configPath: string,
): Promise<DevupConfig> {
  if (!existsSync(configPath)) {
    return {}
  }

  const content = await readFile(configPath, 'utf-8')
  const config = parseConfig(content)

  // If no extends, return the config as-is
  if (!config.extends || config.extends.length === 0) {
    return config
  }

  const configDir = dirname(configPath)

  // Start with empty base and merge extends in order
  // First extend is the base, subsequent ones override
  let mergedConfig: DevupConfig = {}

  for (const extendPath of config.extends) {
    const resolvedPath = resolve(configDir, extendPath)
    const extendedConfig = await loadDevupConfig(resolvedPath)
    mergedConfig = deepMerge(mergedConfig, extendedConfig)
  }

  // Finally, merge the current config (highest priority)
  // Remove extends from the result as it's already resolved
  const { extends: _, ...currentConfig } = config
  return deepMerge(mergedConfig, currentConfig)
}
