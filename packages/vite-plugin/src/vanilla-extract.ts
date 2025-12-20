import type { PluginOption } from 'vite'

import type { VanillaExtractConfig, VanillaExtractOptions } from './plugin'

/**
 * Creates a vanilla-extract configuration for DevupUI
 *
 * @param options - Vanilla Extract options
 * @returns VanillaExtractConfig object to pass to DevupUI plugin
 *
 * @example
 * ```ts
 * import { DevupUI } from '@devup-ui/vite-plugin'
 * import { createVanillaExtractConfig } from '@devup-ui/vite-plugin/vanilla-extract'
 *
 * export default defineConfig({
 *   plugins: [
 *     DevupUI({
 *       vanillaExtract: createVanillaExtractConfig({
 *         identifiers: 'short'
 *       })
 *     })
 *   ]
 * })
 * ```
 */
export function createVanillaExtractConfig(
  options?: VanillaExtractOptions,
): VanillaExtractConfig {
  // Dynamic import to avoid bundling vanilla-extract if not used
  const loadVanillaExtractPlugin = async (): Promise<PluginOption> => {
    const { vanillaExtractPlugin } =
      await import('@vanilla-extract/vite-plugin')
    return vanillaExtractPlugin(options)
  }

  return {
    plugin: loadVanillaExtractPlugin() as any,
    options,
  }
}

/**
 * Type-safe wrapper to enable vanilla-extract with default options
 *
 * @returns VanillaExtractConfig with default settings
 *
 * @example
 * ```ts
 * import { DevupUI } from '@devup-ui/vite-plugin'
 * import { withVanillaExtract } from '@devup-ui/vite-plugin/vanilla-extract'
 *
 * export default defineConfig({
 *   plugins: [
 *     DevupUI({
 *       vanillaExtract: withVanillaExtract()
 *     })
 *   ]
 * })
 * ```
 */
export function withVanillaExtract(
  options?: VanillaExtractOptions,
): VanillaExtractConfig {
  return createVanillaExtractConfig(options)
}
