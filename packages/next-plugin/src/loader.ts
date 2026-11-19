import { existsSync, readFileSync } from 'node:fs'
import { writeFile } from 'node:fs/promises'
import { basename, dirname, extname, join, relative, resolve } from 'node:path'

import {
  createStateWriter,
  type DevupCompiler,
  type DevupEngine,
  tailwindCssFiles,
  withTailwindCss,
} from '@devup-ui/plugin-utils'
import type { RawLoaderDefinitionFunction } from 'webpack'

import { takeExtractOutput } from './coordinator'
import { callCoordinator, resetCoordinatorClient } from './coordinator-client'
import { loadWasm } from './wasm'

const stateWriter = createStateWriter((path, content, encoding) =>
  encoding ? writeFile(path, content, encoding) : writeFile(path, content),
)

/** Options of a loader that sends every file to the coordinator (Turbopack). */
export interface DevupUICoordinatorLoaderOptions {
  coordinatorPortFile: string
  /** Every file the theme is read from, relative to the project root or absolute */
  themeFiles?: string[]
  /** Changes with the setup state, so Turbopack re-runs the loader then */
  stateKey?: string
}

/** Options of a loader that extracts in its own process. */
export interface DevupUIStandaloneLoaderOptions {
  engine?: DevupEngine
  package: string
  cssDir: string
  sheetFile: string
  classMapFile: string
  fileMapFile: string
  themeFile: string
  /** Every file the theme is read from (devup.json, its extends, Tailwind CSS) */
  themeFiles?: string[]
  watch: boolean
  singleCss: boolean
  coordinatorPortFile?: undefined
  theme?: object
  defaultSheet: object
  defaultClassMap: object
  defaultFileMap: object
  importAliases?: Record<string, string | null>
}

export type DevupUILoaderOptions =
  DevupUICoordinatorLoaderOptions | DevupUIStandaloneLoaderOptions

function isCoordinatorOptions(
  options: DevupUILoaderOptions,
): options is DevupUICoordinatorLoaderOptions {
  return !!options.coordinatorPortFile
}
let initializedCompilers = new WeakSet<DevupCompiler>()

interface CoordinatorResponse {
  code?: string
  error?: string
  map?: string
  dependencies: string[]
}

function toLoaderError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error))
}

function parseCoordinatorResponse(content: string): CoordinatorResponse {
  const data: unknown = JSON.parse(content)
  if (typeof data !== 'object' || data === null) {
    return { dependencies: [] }
  }

  const record = data as Record<string, unknown>
  return {
    code: typeof record.code === 'string' ? record.code : undefined,
    error: typeof record.error === 'string' ? record.error : undefined,
    map: typeof record.map === 'string' ? record.map : undefined,
    dependencies: Array.isArray(record.dependencies)
      ? record.dependencies.filter(
          (dependency): dependency is string => typeof dependency === 'string',
        )
      : [],
  }
}

/**
 * The file on disk a module came from. A Turbopack rule with `as` hands later
 * rules the module under a renamed path: `@next/mdx` compiles `page.mdx` with
 * `as: '*.tsx'`, so the source rule receives the compiled JavaScript as
 * `page.mdx.tsx`, which does not exist. Extracting under that name would make
 * the coordinator drop its styles as a deleted source, so the original file
 * names the module instead; the extractor reads `.mdx` as compiled MDX.
 */
export function sourceFilePath(resourcePath: string): string {
  if (!/\.mdx\.[^./\\]+$/i.test(resourcePath) || existsSync(resourcePath))
    return resourcePath
  const original = resourcePath.slice(
    0,
    resourcePath.length - extname(resourcePath).length,
  )
  return existsSync(original) ? original : resourcePath
}

function parseSourceMap(sourceMap: string | undefined): string | null {
  if (!sourceMap) return null

  JSON.parse(sourceMap)
  return sourceMap
}

function coordinatorExtract(
  portFile: string,
  body: string,
  callback: (
    err: Error | null,
    content?: string,
    sourceMap?: string | null,
    dependencies?: string[],
  ) => void,
): void {
  callCoordinator(
    portFile,
    { path: '/extract', method: 'POST', body },
    (error, reply) => {
      if (reply === undefined) {
        callback(error)
        return
      }
      try {
        const data = parseCoordinatorResponse(reply.body)
        if (reply.statusCode !== 200) {
          callback(new Error(data.error ?? 'Coordinator error'))
          return
        }
        if (data.code === undefined) {
          callback(new Error('Coordinator response missing code'))
          return
        }
        const sourceMap = parseSourceMap(data.map)
        callback(null, data.code, sourceMap, data.dependencies)
      } catch (e) {
        callback(toLoaderError(e))
      }
    },
  )
}

const devupUILoader: RawLoaderDefinitionFunction<DevupUILoaderOptions> =
  function (source) {
    const options = this.getOptions()

    // Coordinator mode: delegate to HTTP server
    if (isCoordinatorOptions(options)) {
      const { coordinatorPortFile, themeFiles = [] } = options
      this.addDependency(coordinatorPortFile)
      // Extraction reads theme tokens: re-run when the theme changes (the
      // coordinator registers the new theme before it answers).
      for (const file of themeFiles) this.addDependency(resolve(file))
      const callback = this.async()
      // POSIX-normalize so the engine's bucket key matches the canonical map
      // and FILE_ROUTES keys (both built with forward slashes). Without this,
      // canonical collapse and atom hoisting silently no-op on Windows.
      const resourcePath = sourceFilePath(this.resourcePath)
      const relativePath = relative(process.cwd(), resourcePath).replaceAll(
        '\\',
        '/',
      )
      const body = JSON.stringify({
        filename: relativePath,
        code: source.toString(),
        resourcePath,
      })
      coordinatorExtract(
        coordinatorPortFile,
        body,
        (err, content, sourceMap, dependencies = []) => {
          if (err) return callback(err)
          for (const dependency of dependencies) {
            this.addDependency(resolve(dependency))
          }
          callback(null, content, sourceMap as Parameters<typeof callback>[2])
        },
      )
      return
    }

    const {
      watch,
      engine,
      package: libPackage,
      cssDir,
      sheetFile,
      classMapFile,
      fileMapFile,
      themeFile,
      singleCss,
      theme,
      defaultClassMap,
      defaultFileMap,
      defaultSheet,
      importAliases = {},
    } = options

    // Non-coordinator mode retains one selected compiler through callbacks.
    const callback = this.async()
    try {
      const compiler = loadWasm(engine)
      const {
        codeExtract,
        exportClassMap,
        exportFileMap,
        exportSheet,
        getCss,
        importClassMap,
        importFileMap,
        importSheet,
        registerTheme,
      } = compiler
      const promises: Promise<void>[] = []
      if (!initializedCompilers.has(compiler)) {
        if (watch) {
          this.addDependency(sheetFile)
          this.addDependency(classMapFile)
          this.addDependency(fileMapFile)
          this.addDependency(themeFile)
          // restart loader issue
          // loader should read files when they exist in watch mode
          if (existsSync(sheetFile))
            importSheet(JSON.parse(readFileSync(sheetFile, 'utf-8')))
          if (existsSync(classMapFile))
            importClassMap(JSON.parse(readFileSync(classMapFile, 'utf-8')))
          if (existsSync(fileMapFile))
            importFileMap(JSON.parse(readFileSync(fileMapFile, 'utf-8')))
          if (existsSync(themeFile)) {
            const config = JSON.parse(readFileSync(themeFile, 'utf-8')) ?? {}
            registerTheme(withTailwindCss(config.theme ?? {}, config))
            for (const file of tailwindCssFiles(config))
              this.addDependency(file)
          }
        } else {
          importFileMap(defaultFileMap)
          importClassMap(defaultClassMap)
          importSheet(defaultSheet)
          registerTheme(theme)
        }
        initializedCompilers.add(compiler)
      }
      const id = sourceFilePath(this.resourcePath)
      let relCssDir = relative(dirname(id), cssDir).replaceAll('\\', '/')

      // POSIX-normalize (see coordinator-mode note above) so bucket keys match
      // the canonical map / FILE_ROUTES on Windows.
      const relativePath = relative(process.cwd(), id).replaceAll('\\', '/')

      if (!relCssDir.startsWith('./')) relCssDir = `./${relCssDir}`
      const snapshot = takeExtractOutput(
        codeExtract(
          relativePath,
          source.toString(),
          libPackage,
          relCssDir,
          singleCss,
          false,
          true,
          importAliases,
        ),
        false,
      )
      const {
        code,
        map,
        cssFile,
        updatedBaseStyle,
        dependencies = [],
      } = snapshot
      for (const dependency of dependencies) {
        this.addDependency(resolve(dependency))
      }
      const sourceMap = parseSourceMap(map)
      if (updatedBaseStyle && watch) {
        // update base style
        promises.push(
          stateWriter.write(
            join(cssDir, 'devup-ui.css'),
            getCss(null, false),
            'utf-8',
          ),
        )
      }
      if (cssFile && watch) {
        // don't write file when build
        promises.push(
          stateWriter.write(
            join(cssDir, basename(cssFile)),
            `/* ${this.resourcePath} ${Date.now()} */`,
          ),
          stateWriter.write(sheetFile, exportSheet()),
          stateWriter.write(classMapFile, exportClassMap()),
          stateWriter.write(fileMapFile, exportFileMap()),
        )
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

/** @internal Reset init state for testing purposes only */
export const resetInit = () => {
  initializedCompilers = new WeakSet<DevupCompiler>()
  resetCoordinatorClient()
}

export { setWasmForTesting } from './wasm'
