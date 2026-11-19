import { readFileSync } from 'node:fs'
import { writeFile } from 'node:fs/promises'
import { basename, dirname, join, relative, resolve } from 'node:path'

import {
  createModuleResolver,
  createStateWriter,
  type DevupCompiler,
  type DevupCompilerOutput,
  type DevupEngine,
  esmImportConditions,
  loadCompiler,
  markerLedgerOf,
  markerLikeTokens,
  type MarkerTransform,
  type UnscannedCause,
  unscannedCause,
} from '@devup-ui/plugin-utils'
import type { LoaderContext, RawLoaderDefinitionFunction } from 'webpack'

import { MARKER_BUILD_INFO } from './component-markers'

export interface DevupUILoaderOptions {
  readonly engine?: DevupEngine
  /** The project root extraction ids are relative to; the cwd when absent */
  readonly root?: string
  package: string
  cssDir: string
  sheetFile: string
  classMapFile: string
  fileMapFile: string
  watch: boolean
  singleCss: boolean
  importAliases?: Record<string, string | null>
}

function toLoaderError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error))
}

type LoaderSourceMap = Exclude<
  Parameters<LoaderContext<DevupUILoaderOptions>['callback']>[2],
  string | undefined
>

function parseSourceMap(sourceMap: string | undefined): LoaderSourceMap {
  if (!sourceMap) return null

  return JSON.parse(sourceMap)
}

const stateWriter = createStateWriter((path, content, encoding) =>
  encoding ? writeFile(path, content, encoding) : writeFile(path, content),
)
const moduleResolvers = new Map<
  string,
  ReturnType<typeof createModuleResolver>
>()

/**
 * Resolve imports to the root-relative ids this loader extracts files under,
 * with the `exports` conditions the bundler resolves the same imports with
 */
function setRootModuleResolver(
  compiler: Pick<DevupCompiler, 'setModuleResolver'>,
  root: string,
  conditions: readonly string[] | undefined,
): void {
  const key = JSON.stringify([root, conditions ?? null])
  let moduleResolver = moduleResolvers.get(key)
  if (!moduleResolver) {
    moduleResolver = createModuleResolver({
      conditions,
      toId: (path) => relative(root, path).replaceAll('\\', '/'),
    })
    moduleResolvers.set(key, moduleResolver)
  }
  compiler.setModuleResolver(moduleResolver)
}

/**
 * Tell the build's marker ledger which component-selector markers the module
 * gives and selects, and, for one that selects any, why the start-of-build
 * scan may have missed it
 */
function reportMarkers(
  context: LoaderContext<DevupUILoaderOptions>,
  id: string,
  input: string,
  output: Pick<DevupCompilerOutput, 'givenMarkers' | 'selectedMarkers'>,
): void {
  const owner = context._compiler
  const ledger = markerLedgerOf(owner?.root ?? owner)
  if (!ledger) return
  const selectedMarkers = output.selectedMarkers
  const transform: MarkerTransform = {
    id,
    // Of the code, the ledger reads only the tokens written like a marker
    input: [...markerLikeTokens(input)].join(' '),
    givenMarkers: [...output.givenMarkers],
    selectedMarkers: [...selectedMarkers],
    cause: selectedMarkers.length ? markerCause(context, id, input) : undefined,
  }
  ledger.record(transform)
  // A build that restores the module from webpack's cache runs no loader for
  // it; the plugin reads the report from the module instead
  const buildInfo = context._module?.buildInfo
  if (buildInfo) buildInfo[MARKER_BUILD_INFO] = transform
}

function markerCause(
  context: LoaderContext<DevupUILoaderOptions>,
  id: string,
  input: string,
): UnscannedCause {
  const graph = context._compilation?.moduleGraph
  const module = context._module
  if (
    graph &&
    module &&
    [...graph.getIncomingConnections(module)].some((connection) =>
      connection.dependency?.type.includes('context'),
    )
  )
    return 'context-module'
  let onDisk: string | undefined
  try {
    onDisk = readFileSync(context.resourcePath, 'utf-8')
  } catch {
    onDisk = undefined
  }
  return unscannedCause(id, input, onDisk)
}

const devupUILoader: RawLoaderDefinitionFunction<DevupUILoaderOptions> =
  function (source) {
    const {
      watch,
      package: libPackage,
      cssDir,
      sheetFile,
      classMapFile,
      fileMapFile,
      singleCss,
      engine,
      root = process.cwd(),
      importAliases = {},
    } = this.getOptions()
    const callback = this.async()
    const id = this.resourcePath

    if (watch) {
      this.addDependency(sheetFile)
      this.addDependency(classMapFile)
      this.addDependency(fileMapFile)
    }

    try {
      const compiler = loadCompiler(engine)
      let relCssDir = relative(dirname(id), cssDir).replaceAll('\\', '/')

      // POSIX-normalize so the engine's bucket key matches the canonical map /
      // FILE_ROUTES keys (built with forward slashes by plugin-utils). Without
      // this, single-importer collapse and atom hoisting silently no-op on
      // Windows. No-op on POSIX.
      const relativePath = relative(root, id).replaceAll('\\', '/')

      if (!relCssDir.startsWith('./')) relCssDir = `./${relCssDir}`
      setRootModuleResolver(
        compiler,
        root,
        esmImportConditions(this._compiler?.options.resolve),
      )
      const output: DevupCompilerOutput = compiler.codeExtract(
        relativePath,
        source.toString(),
        libPackage,
        relCssDir,
        singleCss,
        false,
        true,
        importAliases,
      )
      let snapshot: Pick<
        DevupCompilerOutput,
        'code' | 'css' | 'map' | 'cssFile' | 'updatedBaseStyle' | 'dependencies'
      >
      try {
        reportMarkers(this, relativePath, source.toString(), output)
        const cssFile = output.cssFile
        snapshot = {
          code: output.code,
          // The engine serializes the CSS only when it is read; watch mode
          // never writes it
          css: cssFile && !watch ? output.css : undefined,
          map: output.map,
          cssFile,
          updatedBaseStyle: output.updatedBaseStyle,
          dependencies: output.dependencies,
        }
      } finally {
        output.free()
      }
      const {
        code,
        css = '',
        map,
        cssFile,
        updatedBaseStyle,
        dependencies,
      } = snapshot
      for (const dependency of dependencies) {
        this.addDependency(resolve(root, dependency))
      }
      const sourceMap = parseSourceMap(map)
      const promises: Promise<void>[] = []
      if (updatedBaseStyle) {
        // update base style
        promises.push(
          stateWriter.write(
            join(cssDir, 'devup-ui.css'),
            compiler.getCss(null, false),
            'utf-8',
          ),
        )
      }
      if (cssFile) {
        const content = `${basename(cssFile)} ${Date.now()}`
        // should be reset css
        promises.push(
          stateWriter.write(
            join(cssDir, basename(cssFile)),
            watch ? `/* ${content} */` : css,
          ),
        )
        if (watch) {
          promises.push(
            stateWriter.write(sheetFile, compiler.exportSheet()),
            stateWriter.write(classMapFile, compiler.exportClassMap()),
            stateWriter.write(fileMapFile, compiler.exportFileMap()),
          )
        }
      }
      Promise.all(promises).then(
        () => callback(null, code, sourceMap),
        (error) => callback(toLoaderError(error)),
      )
    } catch (error) {
      callback(toLoaderError(error))
    }
    return
  }
export default devupUILoader
