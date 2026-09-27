import { existsSync, realpathSync } from 'node:fs'
import { createRequire } from 'node:module'
import { join, relative } from 'node:path'

import { createModuleResolver } from '@devup-ui/plugin-utils'

export type DevupWasm = typeof import('@devup-ui/wasm')
export type DevupWebpackPlugin = typeof import('@devup-ui/webpack-plugin')

let wasmForTesting: DevupWasm | undefined
let webpackPluginForTesting: DevupWebpackPlugin | undefined
let fullWasm: DevupWasm | undefined
let liteWasm: DevupWasm | undefined
let webpackPlugin: DevupWebpackPlugin | undefined

/** @internal Resolve dependencies from the plugin's physical install location. */
export function requireFromPlugin<T>(specifier: string): T {
  const installedPackage = join(
    process.cwd(),
    'node_modules/@devup-ui/next-plugin/package.json',
  )
  const workspacePackage = join(
    process.cwd(),
    'packages/next-plugin/package.json',
  )
  const requireBase = existsSync(installedPackage)
    ? installedPackage
    : existsSync(workspacePackage)
      ? workspacePackage
      : join(process.cwd(), 'package.json')
  return createRequire(realpathSync(requireBase))(specifier) as T
}

type WasmLoader = (specifier: string) => DevupWasm

function isMissingLiteWasm(error: unknown): boolean {
  if (!(error instanceof Error) || !('code' in error)) return false
  if (error.code === 'ERR_PACKAGE_PATH_NOT_EXPORTED') {
    return (
      error.message.includes("Package subpath './lite'") &&
      error.message.includes('@devup-ui') &&
      error.message.includes('wasm')
    )
  }
  return (
    error.code === 'MODULE_NOT_FOUND' &&
    error.message.includes('@devup-ui/wasm/lite')
  )
}

/** @internal Resolve the lite engine, falling back for older WASM packages. */
export function requireWasm(
  lite: boolean,
  requireModule: WasmLoader = (specifier) =>
    requireFromPlugin<DevupWasm>(specifier),
): DevupWasm {
  if (!lite) return requireModule('@devup-ui/wasm')
  try {
    return requireModule('@devup-ui/wasm/lite')
  } catch (error) {
    if (!isMissingLiteWasm(error)) throw error
    return requireModule('@devup-ui/wasm')
  }
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

/** Load exactly one extraction engine for the lifetime of a Next config. */
export function loadWasm(lite: boolean): DevupWasm {
  if (wasmForTesting) return wasmForTesting
  if (lite) {
    return (liteWasm ??= withModuleResolver(requireWasm(true)))
  }
  return (fullWasm ??= withModuleResolver(requireWasm(false)))
}

/** Keep the Webpack adapter (and its full WASM) out of Turbopack startup. */
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
