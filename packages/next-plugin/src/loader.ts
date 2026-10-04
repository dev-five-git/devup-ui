import { existsSync, readFileSync } from 'node:fs'
import { writeFile } from 'node:fs/promises'
import { basename, dirname, join, relative, resolve } from 'node:path'

import {
  collectDevupConfigFiles,
  createStateWriter,
} from '@devup-ui/plugin-utils'
import type { RawLoaderDefinitionFunction } from 'webpack'

import {
  CoordinatorRequestError,
  requestCoordinator,
} from './coordinator-client'
import type { CoordinatorIdentity } from './coordinator-port'
import { loadWasm } from './wasm'

const stateWriter = createStateWriter((path, content, encoding) =>
  encoding ? writeFile(path, content, encoding) : writeFile(path, content),
)

export interface DevupUILoaderOptions {
  package: string
  cssDir: string
  sheetFile: string
  classMapFile: string
  fileMapFile: string
  themeFile: string
  watch: boolean
  singleCss: boolean
  coordinatorPortFile?: string
  coordinatorIdentity?: CoordinatorIdentity
  projectRoot?: string
  revisionFile?: string
  themeFiles?: string[]
  requestTimeoutMs?: number
  theme?: object
  defaultSheet: object
  defaultClassMap: object
  defaultFileMap: object
  importAliases?: Record<string, string | null>
}
let init = false

function toLoaderError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error))
}

function parseCoordinatorResponse(content: string) {
  const data: unknown = JSON.parse(content)
  if (
    typeof data !== 'object' ||
    data === null ||
    !('code' in data) ||
    typeof data.code !== 'string'
  ) {
    throw new Error('Coordinator response missing code')
  }
  const map =
    'map' in data && typeof data.map === 'string' ? data.map : undefined
  return {
    code: data.code,
    map: parseSourceMap(map),
    dependencies:
      'dependencies' in data && Array.isArray(data.dependencies)
        ? data.dependencies.filter(
            (dependency): dependency is string =>
              typeof dependency === 'string',
          )
        : [],
  }
}

function parseSourceMap(sourceMap: string | undefined): string | null {
  if (!sourceMap) return null
  JSON.parse(sourceMap)
  return sourceMap
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
      themeFile,
      singleCss,
      coordinatorPortFile,
      coordinatorIdentity,
      projectRoot = process.cwd(),
      revisionFile,
      themeFiles,
      requestTimeoutMs,
      theme,
      defaultClassMap,
      defaultFileMap,
      defaultSheet,
      importAliases = {},
    } = this.getOptions()
    const callback = this.async()
    if (coordinatorPortFile) {
      const operation = {
        portFile: coordinatorPortFile,
        identity: coordinatorIdentity,
        resourcePath: this.resourcePath,
        path: '/extract',
        method: 'POST' as const,
        timeoutMs: requestTimeoutMs,
        body: JSON.stringify({
          filename: relative(projectRoot, this.resourcePath).replaceAll(
            '\\',
            '/',
          ),
          code: source.toString(),
          resourcePath: this.resourcePath,
        }),
      }
      try {
        this.addDependency(coordinatorPortFile)
        if (revisionFile) this.addDependency(resolve(projectRoot, revisionFile))
        for (const file of themeFiles ??
          collectDevupConfigFiles(resolve(projectRoot, themeFile))) {
          this.addDependency(resolve(projectRoot, file))
        }
      } catch (error) {
        callback(new CoordinatorRequestError(operation, error))
        return
      }
      requestCoordinator(operation)
        .then((content) => {
          const data = parseCoordinatorResponse(content)
          for (const dependency of data.dependencies)
            this.addDependency(resolve(projectRoot, dependency))
          return data
        })
        .then(
          (data) => callback(null, data.code, data.map),
          (error: unknown) =>
            callback(
              error instanceof CoordinatorRequestError
                ? error
                : new CoordinatorRequestError(operation, error),
            ),
        )
      return
    }

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
    } = loadWasm()
    const promises: Promise<void>[] = []
    try {
      if (!init) {
        if (watch) {
          this.addDependency(sheetFile)
          this.addDependency(classMapFile)
          this.addDependency(fileMapFile)
          this.addDependency(themeFile)
          const sheet = existsSync(sheetFile)
            ? JSON.parse(readFileSync(sheetFile, 'utf-8'))
            : undefined
          const classes = existsSync(classMapFile)
            ? JSON.parse(readFileSync(classMapFile, 'utf-8'))
            : undefined
          const files = existsSync(fileMapFile)
            ? JSON.parse(readFileSync(fileMapFile, 'utf-8'))
            : undefined
          const config = existsSync(themeFile)
            ? JSON.parse(readFileSync(themeFile, 'utf-8'))
            : undefined
          if (sheet !== undefined) importSheet(sheet)
          if (classes !== undefined) importClassMap(classes)
          if (files !== undefined) importFileMap(files)
          if (config !== undefined) registerTheme(config?.theme ?? {})
        } else {
          importFileMap(defaultFileMap)
          importClassMap(defaultClassMap)
          importSheet(defaultSheet)
          registerTheme(theme)
        }
        init = true
      }
      const id = this.resourcePath
      let relCssDir = relative(dirname(id), cssDir).replaceAll('\\', '/')
      const relativePath = relative(projectRoot, id).replaceAll('\\', '/')
      if (!relCssDir.startsWith('./')) relCssDir = `./${relCssDir}`
      const {
        code,
        map,
        cssFile,
        updatedBaseStyle,
        dependencies = [],
      } = codeExtract(
        relativePath,
        source.toString(),
        libPackage,
        relCssDir,
        singleCss,
        false,
        true,
        importAliases,
      )
      for (const dependency of dependencies)
        this.addDependency(resolve(projectRoot, dependency))
      const sourceMap = parseSourceMap(map)
      if (updatedBaseStyle && watch) {
        promises.push(
          stateWriter.write(
            join(cssDir, 'devup-ui.css'),
            getCss(null, false),
            'utf-8',
          ),
        )
      }
      if (cssFile && watch) {
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
        (error: unknown) => callback(toLoaderError(error)),
      )
    } catch (error) {
      callback(toLoaderError(error))
    }
  }
export default devupUILoader

export const resetInit = () => {
  init = false
}
export { setWasmForTesting } from './wasm'
