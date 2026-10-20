import { existsSync, realpathSync } from 'node:fs'
import { createRequire } from 'node:module'
import { join, relative } from 'node:path'

import { createModuleResolver } from '@devup-ui/plugin-utils'

export type DevupWasm = typeof import('@devup-ui/wasm')
export type DevupWebpackPlugin = typeof import('@devup-ui/webpack-plugin')

let wasmForTesting: DevupWasm | undefined
let webpackPluginForTesting: DevupWebpackPlugin | undefined
let loadedWasm: DevupWasm | undefined
let webpackPlugin: DevupWebpackPlugin | undefined

/** @internal Resolve dependencies from the plugin's physical install location. */
export function requireFromPlugin<T>(
  specifier: string,
  root = process.cwd(),
): T {
  const installedPackage = join(
    root,
    'node_modules/@devup-ui/next-plugin/package.json',
  )
  const workspacePackage = join(root, 'packages/next-plugin/package.json')
  const requireBase = existsSync(installedPackage)
    ? installedPackage
    : existsSync(workspacePackage)
      ? workspacePackage
      : join(root, 'package.json')
  return createRequire(realpathSync(requireBase))(specifier) as T
}

/**
 * Resolve the imports of extracted files to the cwd-relative ids every Next
 * extraction path passes, on engines new enough to load modules.
 */
export function withModuleResolver(wasm: DevupWasm): DevupWasm {
  if ('setModuleResolver' in wasm) {
    wasm.setModuleResolver(
      createModuleResolver({
        toId: (path) => relative(process.cwd(), path).replaceAll('\\', '/'),
      }),
    )
  }
  return wasm
}

/** Load the extraction engine once for the lifetime of a Next config. */
export function loadWasm(): DevupWasm {
  if (wasmForTesting) return wasmForTesting
  return (loadedWasm ??= withModuleResolver(
    requireFromPlugin<DevupWasm>('@devup-ui/wasm'),
  ))
}

/** Keep the Webpack adapter out of Turbopack startup. */
export function loadWebpackPlugin(): DevupWebpackPlugin {
  if (webpackPluginForTesting) return webpackPluginForTesting
  return (webpackPlugin ??= requireFromPlugin<DevupWebpackPlugin>(
    '@devup-ui/webpack-plugin',
  ))
}

/** @internal Inject the WASM namespace for unit tests. */
export function setWasmForTesting(value: DevupWasm | undefined): void {
  wasmForTesting = value
}

/** @internal Inject the Webpack namespace for unit tests. */
export function setWebpackPluginForTesting(
  value: DevupWebpackPlugin | undefined,
): void {
  webpackPluginForTesting = value
}

/** @internal Clear overrides and cached namespaces between tests. */
export function resetWasmForTesting(): void {
  wasmForTesting = undefined
  webpackPluginForTesting = undefined
  loadedWasm = undefined
  webpackPlugin = undefined
}
