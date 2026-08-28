import { existsSync } from 'node:fs'
import { createRequire } from 'node:module'
import { join } from 'node:path'

export type DevupWasm = typeof import('@devup-ui/wasm')
export type DevupWebpackPlugin = typeof import('@devup-ui/webpack-plugin')

let wasmForTesting: DevupWasm | undefined
let webpackPluginForTesting: DevupWebpackPlugin | undefined
let fullWasm: DevupWasm | undefined
let liteWasm: DevupWasm | undefined
let webpackPlugin: DevupWebpackPlugin | undefined

function requireFromPlugin<T>(specifier: string): T {
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
  return createRequire(requireBase)(specifier) as T
}

/** Load exactly one extraction engine for the lifetime of a Next config. */
export function loadWasm(lite: boolean): DevupWasm {
  if (wasmForTesting) return wasmForTesting
  if (lite) {
    return (liteWasm ??= requireFromPlugin<DevupWasm>('@devup-ui/wasm/lite'))
  }
  return (fullWasm ??= requireFromPlugin<DevupWasm>('@devup-ui/wasm'))
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
