import { existsSync, readFileSync, realpathSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, join, relative, resolve } from 'node:path'
import { compileFunction } from 'node:vm'

import {
  createModuleResolver,
  type ModuleAliases,
  type PrepareSource,
} from '@devup-ui/plugin-utils'

export type DevupWasm = typeof import('@devup-ui/wasm')
export type DevupWebpackPlugin = typeof import('@devup-ui/webpack-plugin')

export interface ModuleResolverSettings {
  readonly prepareSource?: PrepareSource
  readonly alias?: ModuleAliases
  readonly includeMdx?: boolean | readonly string[]
  readonly conditions?: readonly string[]
}

const engineResolvers = new WeakMap<
  DevupWasm,
  {
    readonly root: string
    readonly settings: ModuleResolverSettings | undefined
    readonly resolver: ReturnType<typeof createModuleResolver>
  }
>()

let wasmForTesting: DevupWasm | undefined
let webpackPluginForTesting: DevupWebpackPlugin | undefined
let loadedWasm: DevupWasm | undefined
let webpackPlugin: DevupWebpackPlugin | undefined

/** @internal Resolve dependencies from the plugin's physical install location. */
export function requireFromPlugin<T>(specifier: string): T {
  return createPluginRequire(process.cwd())(specifier) as T
}

function createPluginRequire(projectRoot: string): NodeRequire {
  const installedPackage = join(
    projectRoot,
    'node_modules/@devup-ui/next-plugin/package.json',
  )
  const workspacePackage = join(
    projectRoot,
    'packages/next-plugin/package.json',
  )
  const requireBase = existsSync(installedPackage)
    ? installedPackage
    : existsSync(workspacePackage)
      ? workspacePackage
      : join(projectRoot, 'package.json')
  return createRequire(realpathSync(requireBase))
}

/**
 * Resolve the imports of extracted files to the root-relative ids every Next
 * extraction path passes, on engines new enough to load modules.
 */
export function withModuleResolver(
  wasm: DevupWasm,
  projectRoot = process.cwd(),
  settings?: ModuleResolverSettings,
): DevupWasm {
  const root = resolve(projectRoot)
  if ('setModuleResolver' in wasm) {
    const previous = engineResolvers.get(wasm)
    if (
      settings !== undefined &&
      previous?.root === root &&
      previous.settings === settings
    )
      return wasm
    const resolver = createModuleResolver({
      ...settings,
      cwd: root,
      toId: (path) => relative(root, path).replaceAll('\\', '/'),
    })
    wasm.setModuleResolver(resolver)
    engineResolvers.set(wasm, { root, settings, resolver })
  }
  return wasm
}

/** Shared by requests, replay, sealed candidates, prewarm and legacy loaders. */
export function extractWithModuleResolver(
  wasm: DevupWasm,
  sourceMap: boolean,
  args: Parameters<DevupWasm['codeExtract']>,
): ReturnType<DevupWasm['codeExtract']> {
  const configured = engineResolvers.get(wasm)
  try {
    // Register the request's own compiler map, not just imported module maps.
    const prepared = configured?.settings?.prepareSource
      ? configured.resolver(resolve(configured.root, args[0]), args[0])
      : undefined
    const extract = sourceMap
      ? wasm.codeExtract
      : wasm.codeExtractWithoutSourceMap
    const sourceType = args[8] ?? prepared?.sourceType
    const extractionArgs: Parameters<DevupWasm['codeExtract']> = [...args]
    if (sourceType !== undefined) extractionArgs[8] = sourceType
    return extract(...extractionArgs)
  } catch (error) {
    throw configured?.settings?.prepareSource
      ? configured.resolver.remapError(error)
      : error
  }
}

/** Evaluate a fresh bridge and WASM instance for one app, in the caller's realm. */
export function createWasm(projectRoot = process.cwd()): DevupWasm {
  if (wasmForTesting) return wasmForTesting
  const root = resolve(projectRoot)
  const filename = createPluginRequire(root).resolve('@devup-ui/wasm')
  // Only the entry is private: its closure owns the bridge heap and Rust state.
  const namespace: DevupWasm = Object.create(null)
  const module = { exports: namespace }
  compileFunction(
    readFileSync(filename, 'utf8'),
    ['exports', 'require', 'module', '__filename', '__dirname'],
    { filename },
  ).call(
    namespace,
    namespace,
    createRequire(filename),
    module,
    filename,
    dirname(filename),
  )
  return withModuleResolver(module.exports, root)
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
