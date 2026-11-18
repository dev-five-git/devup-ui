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
 * Parse a config file's content. A file that is not a JSON object is an
 * error naming the file: loading it as an empty config would silently drop
 * the whole theme.
 */
function parseConfig(content: string, path: string): DevupConfig {
  let config: unknown
  try {
    config = JSON.parse(content)
  } catch (cause) {
    throw new Error(
      `${path} is not valid JSON: ${(cause as Error).message}. Fix its syntax; Devup UI does not load the theme until it parses.`,
      { cause },
    )
  }
  if (!isPlainObject(config)) {
    throw new Error(
      `${path} must hold a JSON object such as { "theme": {} }, not ${config === null ? 'null' : Array.isArray(config) ? 'an array' : `a ${typeof config}`}.`,
    )
  }
  return config as DevupConfig
}

/**
 * The `extends` chain that leads to `configPath`, ending with it. A file
 * that is already on the chain extends itself through it, which never ends.
 */
function enterConfig(configPath: string, chain: readonly string[]): string[] {
  const path = resolve(configPath)
  if (chain.includes(path)) {
    throw new Error(
      `devup.json \`extends\` form a cycle: ${[...chain, path].join(' -> ')}. Remove the \`extends\` entry that points back to an earlier file.`,
    )
  }
  return [...chain, path]
}

/**
 * The config of a file that does not exist: empty. A project without
 * devup.json is normal, but an `extends` entry naming a missing file is
 * likely a typo, so that case warns, naming both files. The path stays in the
 * inputs, so a watcher picks the file up once it is created.
 */
function missingConfig(chain: readonly string[]): DevupConfig {
  if (chain.length > 1) {
    console.warn(
      `[devup-ui] ${chain[chain.length - 2]} extends ${chain[chain.length - 1]}, which does not exist, so it adds nothing to the theme. Create the file or fix the \`extends\` entry.`,
    )
  }
  return {}
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
  inputs?: Set<string>,
): DevupConfig {
  return loadConfigSync(configPath, inputs, [])
}

function loadConfigSync(
  configPath: string,
  inputs: Set<string> | undefined,
  parents: readonly string[],
): DevupConfig {
  inputs?.add(resolve(configPath))
  const chain = enterConfig(configPath, parents)
  if (!existsSync(configPath)) {
    return missingConfig(chain)
  }

  const content = readFileSync(configPath, 'utf-8')
  const config = parseConfig(content, chain[chain.length - 1])

  // If no extends, return the config as-is
  if (!config.extends || config.extends.length === 0) {
    return config
  }

  const configDir = dirname(configPath)

  return mergeExtendedConfigs(
    config,
    config.extends.map((extendPath) =>
      loadConfigSync(resolve(configDir, extendPath), inputs, chain),
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
export function loadDevupConfig(
  configPath: string,
  inputs?: Set<string>,
): Promise<DevupConfig> {
  return loadConfig(configPath, inputs, [])
}

async function loadConfig(
  configPath: string,
  inputs: Set<string> | undefined,
  parents: readonly string[],
): Promise<DevupConfig> {
  inputs?.add(resolve(configPath))
  const chain = enterConfig(configPath, parents)
  if (!existsSync(configPath)) {
    return missingConfig(chain)
  }

  const content = await readFile(configPath, 'utf-8')
  const config = parseConfig(content, chain[chain.length - 1])

  // If no extends, return the config as-is
  if (!config.extends || config.extends.length === 0) {
    return config
  }

  const configDir = dirname(configPath)

  // Load extends sequentially to preserve merge order
  const extendedConfigs: DevupConfig[] = []
  for (const extendPath of config.extends) {
    extendedConfigs.push(
      await loadConfig(resolve(configDir, extendPath), inputs, chain),
    )
  }

  return mergeExtendedConfigs(config, extendedConfigs)
}
