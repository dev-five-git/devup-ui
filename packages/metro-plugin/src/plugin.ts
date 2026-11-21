import { createRequire } from 'node:module'
import { dirname, isAbsolute, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import {
  type DevupEngine,
  resolveDevupFile,
  RUNTIME_PACKAGE,
} from '@devup-ui/plugin-utils'

import {
  type MetroTransformerSettings,
  readSettings,
  writeSettings,
} from './settings'

export interface DevupUIMetroPluginOptions {
  /**
   * The engine. Native by default; `'wasm'` selects the WebAssembly engine,
   * as in the other plugins.
   */
  engine?: DevupEngine
  /** The package Devup UI is imported from. Defaults to `@devup-ui/react`. */
  package?: string
  /** Path of devup.json, from the project root. Defaults to `devup.json`. */
  devupFile?: string
  /**
   * Packages in `node_modules` whose files are compiled as well: a library
   * that ships Devup UI components uncompiled. Each name matches that whole
   * package (`pkg` is not `pkg-extra` or `@scope/pkg`).
   */
  include?: string[]
  /**
   * Whether the React Native runtime (`@devup-ui/react/native`) may be
   * imported for what needs it: a value that differs by theme, `_theme*` with
   * more than one theme, and `useTheme`, `getTheme` and `setTheme`. Defaults
   * to `true`; `false` keeps every file free of runtime imports and makes
   * those build errors, while a token of a single theme still compiles to its
   * value.
   */
  runtime?: boolean
  /**
   * The Babel transformer the compiled code goes to, as a path or a package
   * resolved from the project root. Defaults to the `babelTransformerPath`
   * of the config, else Expo's transformer, else React Native's.
   */
  upstreamTransformer?: string
}

/** The part of a Metro config this plugin reads and changes */
export interface MetroConfigLike {
  projectRoot?: string
  transformer?: {
    babelTransformerPath?: string
  }
}

/**
 * The package animations and transitions run on: `keyframes()` and the
 * `animation*` and `transition*` props are build errors in a project without it
 */
export const REANIMATED_PACKAGE = 'react-native-reanimated'

/** The transformers tried, in order, when neither the config nor the options name one */
export const DEFAULT_UPSTREAM_TRANSFORMERS = [
  '@expo/metro-config/babel-transformer',
  '@react-native/metro-babel-transformer',
] as const

/** Where this module is; the CJS build substitutes `module.filename` */
function moduleDirectory(location: string): string {
  return dirname(isAbsolute(location) ? location : fileURLToPath(location))
}

/** The Babel transformer Metro runs: the built `transformer.cjs` next to this module */
export const TRANSFORMER_PATH = join(
  moduleDirectory(import.meta.url),
  'transformer.cjs',
)

export class UpstreamTransformerNotFoundError extends Error {
  readonly name = 'UpstreamTransformerNotFoundError'

  constructor(projectRoot: string) {
    super(
      `[devup-ui] Cannot find a Babel transformer for Metro in ${projectRoot}. Install \`@expo/metro-config\` (Expo) or \`@react-native/metro-babel-transformer\` (React Native), or name one with the \`upstreamTransformer\` option of withDevupUI().`,
    )
  }
}

/** `request` resolved from `projectRoot`, as `require` there resolves it */
function resolveFrom(projectRoot: string, request: string): string {
  return createRequire(join(projectRoot, 'package.json')).resolve(request)
}

/** `request` resolved from `projectRoot`, or `undefined` when it is not installed */
function findFrom(projectRoot: string, request: string): string | undefined {
  try {
    return resolveFrom(projectRoot, request)
  } catch {
    return undefined
  }
}

/** The Babel transformer the compiled code goes to */
function upstreamTransformer(
  projectRoot: string,
  configured: string | undefined,
  option: string | undefined,
): string {
  if (option !== undefined) {
    return resolveFrom(projectRoot, option)
  }
  if (configured === TRANSFORMER_PATH) {
    // Wrapped twice: keep the transformer the first call chose
    return readSettings().upstreamTransformer
  }
  if (configured !== undefined) return configured
  for (const request of DEFAULT_UPSTREAM_TRANSFORMERS) {
    const path = findFrom(projectRoot, request)
    if (path !== undefined) return path
  }
  throw new UpstreamTransformerNotFoundError(projectRoot)
}

/**
 * Compile Devup UI for React Native in Metro: the config's Babel transformer
 * becomes Devup UI's, which compiles each file that imports Devup UI and then
 * hands the code to the transformer the config had (Expo's or React
 * Native's).
 *
 * ```js
 * // metro.config.js
 * const { getDefaultConfig } = require('expo/metro-config')
 * const { withDevupUI } = require('@devup-ui/metro-plugin')
 *
 * module.exports = withDevupUI(getDefaultConfig(__dirname))
 * ```
 */
export function withDevupUI<T extends MetroConfigLike>(
  config: T,
  options: DevupUIMetroPluginOptions = {},
): T {
  const projectRoot = config.projectRoot ?? process.cwd()
  const settings: MetroTransformerSettings = {
    upstreamTransformer: upstreamTransformer(
      projectRoot,
      config.transformer?.babelTransformerPath,
      options.upstreamTransformer,
    ),
    package: options.package ?? RUNTIME_PACKAGE,
    include: options.include ?? [],
    runtime: options.runtime ?? true,
    reanimated: findFrom(projectRoot, REANIMATED_PACKAGE) !== undefined,
    devupFile: resolve(
      projectRoot,
      resolveDevupFile(projectRoot, options.devupFile),
    ),
    engine: options.engine,
  }
  writeSettings(settings)
  return {
    ...config,
    transformer: {
      ...config.transformer,
      babelTransformerPath: TRANSFORMER_PATH,
    },
  }
}
