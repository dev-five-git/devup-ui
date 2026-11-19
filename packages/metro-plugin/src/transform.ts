import { createHash } from 'node:crypto'
import { existsSync, readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, join } from 'node:path'

import {
  createNodeModulesExcludeRegex,
  type DevupEngine,
  loadCompiler,
  loadDevupConfigSync,
  SOURCE_FILE_REGEX,
} from '@devup-ui/plugin-utils'

import { locatedError } from './located-error'
import { type MetroTransformerSettings, readSettings } from './settings'
import { remapLocations } from './source-map'

/** What Metro passes a Babel transformer */
export interface BabelTransformerArgs {
  readonly filename: string
  readonly src: string
  readonly options: {
    readonly platform?: string | null
    readonly projectRoot: string
  }
  readonly plugins?: unknown
}

/** A Babel transformer module as Metro loads it */
export interface BabelTransformer {
  transform(args: BabelTransformerArgs): unknown
  getCacheKey?(...args: unknown[]): string
}

/** The part of the engine the transformer calls */
export interface NativeCompiler {
  codeExtractNative(
    filename: string,
    code: string,
    packageName: string,
    platform: string,
    runtime: boolean,
  ): {
    readonly code: string
    readonly map?: string | null
    free(): void
  }
}

/** What the transformer loads, replaceable in tests */
export interface TransformerHost {
  readonly settings: () => MetroTransformerSettings
  readonly requireModule: (path: string) => BabelTransformer
  readonly loadCompiler: (engine?: DevupEngine) => NativeCompiler
  readonly engineVersion: (engine?: DevupEngine) => string
}

/** The package an engine is loaded from */
function enginePackage(engine?: DevupEngine): string {
  return engine === 'wasm' ? '@devup-ui/wasm' : '@devup-ui/binding'
}

/**
 * The `name@version` of the package the engine is loaded from, found from the
 * module `require` resolves it to, or `name@unknown` when it is not installed
 */
export function engineVersion(
  engine: DevupEngine | undefined,
  require: { resolve(request: string): string },
): string {
  const name = enginePackage(engine)
  let directory: string
  try {
    directory = dirname(require.resolve(name))
  } catch {
    return `${name}@unknown`
  }
  for (;;) {
    const manifest = join(directory, 'package.json')
    if (existsSync(manifest)) {
      const { name: found, version } = JSON.parse(
        readFileSync(manifest, 'utf-8'),
      )
      if (found === name) return `${name}@${version}`
    }
    const parent = dirname(directory)
    if (parent === directory) return `${name}@unknown`
    directory = parent
  }
}

const localRequire = createRequire(import.meta.url)

const defaultHost: TransformerHost = {
  settings: () => readSettings(),
  requireModule: (path) => localRequire(path),
  loadCompiler: (engine) => loadCompiler(engine),
  engineVersion: (engine) => engineVersion(engine, localRequire),
}

/** devup.json and the files of its `extends` chain, as they are on disk */
function devupFileContents(devupFile: string): string[] {
  const inputs = new Set<string>()
  try {
    loadDevupConfigSync(devupFile, inputs)
  } catch {
    // An unreadable chain still keys on the files read so far
  }
  return [...inputs].map(
    (path) => `${path}\0${existsSync(path) ? readFileSync(path, 'utf-8') : ''}`,
  )
}

/**
 * The Babel transformer of `@devup-ui/metro-plugin`: it compiles a file that
 * imports Devup UI for the platform Metro builds, then hands the code to the
 * upstream transformer. Everything is loaded on first use, once per worker.
 */
export function createTransformer(host: TransformerHost = defaultHost) {
  let loaded:
    | {
        readonly settings: MetroTransformerSettings
        readonly upstream: BabelTransformer
        readonly exclude: RegExp
      }
    | undefined
  let compiler: NativeCompiler | undefined

  function load() {
    if (!loaded) {
      const settings = host.settings()
      const upstream = host.requireModule(settings.upstreamTransformer)
      loaded = {
        settings,
        upstream,
        exclude: createNodeModulesExcludeRegex([...settings.include]),
      }
    }
    return loaded
  }

  /**
   * `src` compiled for React Native with its source map, or `undefined` when
   * it uses no Devup UI
   */
  function compile({
    filename,
    src,
    options,
  }: BabelTransformerArgs): { code: string; map?: string | null } | undefined {
    const { settings, exclude } = load()
    if (
      !SOURCE_FILE_REGEX.test(filename) ||
      exclude.test(filename) ||
      !src.includes(settings.package)
    ) {
      return undefined
    }
    compiler ??= host.loadCompiler(settings.engine)
    let output: ReturnType<NativeCompiler['codeExtractNative']>
    try {
      output = compiler.codeExtractNative(
        filename,
        src,
        settings.package,
        options.platform ?? '',
        settings.runtime,
      )
    } catch (error) {
      throw locatedError(error, filename, src)
    }
    try {
      return { code: output.code, map: output.map }
    } finally {
      output.free()
    }
  }

  return {
    transform(args: BabelTransformerArgs): unknown {
      const { upstream } = load()
      const compiled = compile(args)
      if (!compiled || compiled.code === args.src)
        return upstream.transform(args)
      const result = upstream.transform({ ...args, src: compiled.code })
      if (
        compiled.map &&
        typeof result === 'object' &&
        result !== null &&
        'ast' in result
      ) {
        remapLocations(result.ast, compiled.map)
      }
      return result
    },

    /**
     * The upstream key, devup.json and its `extends` chain, the engine
     * version and the settings: a change to any of them recompiles every
     * file on the next Metro start. Metro adds the transformer file itself.
     */
    getCacheKey(...args: unknown[]): string {
      const { settings, upstream } = load()
      const hash = createHash('sha256')
      for (const part of [
        upstream.getCacheKey?.(...args) ?? '',
        ...devupFileContents(settings.devupFile),
        host.engineVersion(settings.engine),
        JSON.stringify(settings),
      ]) {
        hash.update(part).update('\0')
      }
      return hash.digest('hex')
    },
  }
}
