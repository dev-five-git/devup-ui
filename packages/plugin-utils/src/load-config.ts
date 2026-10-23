import { readFileSync, realpathSync } from 'node:fs'
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
export class ConfigLoadError extends Error {
  constructor(
    readonly file: string,
    cause: unknown,
  ) {
    super(
      `${file}:1:1: Cannot load configuration: ${cause instanceof Error ? cause.message : String(cause)}`,
      { cause },
    )
    this.name = 'ConfigLoadError'
  }
}

function isDevupConfig(value: unknown): value is DevupConfig {
  return (
    isPlainObject(value) &&
    (value.extends === undefined ||
      (Array.isArray(value.extends) &&
        value.extends.every((path) => typeof path === 'string')))
  )
}

function parseConfig(content: string): DevupConfig {
  const config: unknown = JSON.parse(content)
  if (!isDevupConfig(config))
    throw new TypeError(
      'Expected a config object with an array of extends paths',
    )
  return config
}

function configStack(configPath: string, stack: readonly string[]): string[] {
  const canonical = realpathSync(configPath)
  if (stack.includes(canonical) || stack.length >= 128) {
    throw new ConfigLoadError(
      configPath,
      new Error(
        `Configuration inheritance cycle or depth limit: ${[...stack, canonical].join(' -> ')}`,
      ),
    )
  }
  return [...stack, canonical]
}

/**
 * Merge resolved parent configs with the current config
 * Extends are merged in order (first is the base, subsequent ones override),
 * then the current config is merged last (highest priority) with its
 * already-resolved extends field removed
 */
function mergeExtendedConfigs(
  config: DevupConfig,
  extendedConfigs: DevupConfig[],
): DevupConfig {
  let mergedConfig: DevupConfig = {}

  for (const extendedConfig of extendedConfigs) {
    mergedConfig = deepMerge(mergedConfig, extendedConfig)
  }

  const { extends: _, ...currentConfig } = config
  return deepMerge(mergedConfig, currentConfig)
}

/**
 * Load and resolve a devup.json config file synchronously
 * Handles the extends field by loading and merging parent configs
 *
 * @param configPath - Path to the devup.json file
 * @returns Resolved configuration with all extends merged
 */
export function loadDevupConfigSync(
  configPath: string,
  stack: readonly string[] = [],
): DevupConfig {
  const file = resolve(configPath)
  let config: DevupConfig
  let nextStack: string[]
  try {
    nextStack = configStack(file, stack)
    config = parseConfig(readFileSync(file, 'utf-8'))
  } catch (cause) {
    if (
      stack.length === 0 &&
      cause instanceof Error &&
      'code' in cause &&
      cause.code === 'ENOENT'
    )
      return {}
    if (cause instanceof ConfigLoadError) throw cause
    throw new ConfigLoadError(file, cause)
  }

  // If no extends, return the config as-is
  if (!config.extends || config.extends.length === 0) {
    return config
  }

  const configDir = dirname(file)

  return mergeExtendedConfigs(
    config,
    config.extends.map((extendPath) =>
      loadDevupConfigSync(resolve(configDir, extendPath), nextStack),
    ),
  )
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
  stack: readonly string[] = [],
): Promise<DevupConfig> {
  const file = resolve(configPath)
  let config: DevupConfig
  let nextStack: string[]
  try {
    nextStack = configStack(file, stack)
    config = parseConfig(await readFile(file, 'utf-8'))
  } catch (cause) {
    if (
      stack.length === 0 &&
      cause instanceof Error &&
      'code' in cause &&
      cause.code === 'ENOENT'
    )
      return {}
    if (cause instanceof ConfigLoadError) throw cause
    throw new ConfigLoadError(file, cause)
  }

  // If no extends, return the config as-is
  if (!config.extends || config.extends.length === 0) {
    return config
  }

  const configDir = dirname(file)

  // Load extends sequentially to preserve merge order
  const extendedConfigs: DevupConfig[] = []
  for (const extendPath of config.extends) {
    extendedConfigs.push(
      await loadDevupConfig(resolve(configDir, extendPath), nextStack),
    )
  }

  return mergeExtendedConfigs(config, extendedConfigs)
}
