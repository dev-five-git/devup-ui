import { existsSync } from 'node:fs'
import { createRequire } from 'node:module'
import { join } from 'node:path'

export type DevupWasm = typeof import('@devup-ui/wasm')

let wasmForTesting: DevupWasm | undefined
let fullWasm: DevupWasm | undefined
let liteWasm: DevupWasm | undefined

function requireFromPlugin(specifier: string): DevupWasm {
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
  return createRequire(requireBase)(specifier) as DevupWasm
}

/** Load exactly one extraction engine for the lifetime of a Next config. */
export function loadWasm(lite: boolean): DevupWasm {
  if (wasmForTesting) return wasmForTesting
  if (lite) {
    return (liteWasm ??= requireFromPlugin('@devup-ui/wasm/lite'))
  }
  return (fullWasm ??= requireFromPlugin('@devup-ui/wasm'))
}

/** @internal Inject the WASM namespace for unit tests. */
export function setWasmForTesting(value: DevupWasm | undefined): void {
  wasmForTesting = value
}
