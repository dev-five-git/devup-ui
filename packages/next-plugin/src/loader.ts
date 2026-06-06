import { existsSync, readFileSync } from 'node:fs'
import { writeFile } from 'node:fs/promises'
import { Agent, request } from 'node:http'
import { basename, dirname, join, relative } from 'node:path'

import {
  codeExtract,
  exportClassMap,
  exportFileMap,
  exportSheet,
  getCss,
  importClassMap,
  importFileMap,
  importSheet,
  registerTheme,
} from '@devup-ui/wasm'
import type { RawLoaderDefinitionFunction } from 'webpack'

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
  // turbo
  theme?: object
  defaultSheet: object
  defaultClassMap: object
  defaultFileMap: object
  importAliases?: Record<string, string | null>
}
let init = false

const cachedPorts = new Map<string, number>()
const keepAliveAgent = new Agent({ keepAlive: true })

interface CoordinatorResponse {
  code?: string
  error?: string
  map?: string
}

function toLoaderError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error))
}

function readCoordinatorPort(portFile: string): number {
  const cachedPort = cachedPorts.get(portFile)
  if (cachedPort !== undefined) return cachedPort

  const port = Number.parseInt(readFileSync(portFile, 'utf-8').trim(), 10)
  cachedPorts.set(portFile, port)
  return port
}

function parseCoordinatorResponse(content: string): CoordinatorResponse {
  const data: unknown = JSON.parse(content)
  if (typeof data !== 'object' || data === null) {
    return {}
  }

  const record = data as Record<string, unknown>
  return {
    code: typeof record.code === 'string' ? record.code : undefined,
    error: typeof record.error === 'string' ? record.error : undefined,
    map: typeof record.map === 'string' ? record.map : undefined,
  }
}

function parseSourceMap(sourceMap: string | undefined): string | null {
  if (!sourceMap) return null

  JSON.parse(sourceMap)
  return sourceMap
}

function coordinatorExtract(
  port: number,
  body: string,
  callback: (
    err: Error | null,
    content?: string,
    sourceMap?: string | null,
  ) => void,
): void {
  const req = request(
    {
      hostname: '127.0.0.1',
      port,
      path: '/extract',
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      agent: keepAliveAgent,
    },
    (res) => {
      const chunks: Buffer[] = []
      res.on('data', (chunk: Buffer) => chunks.push(chunk))
      res.on('end', () => {
        try {
          const data = parseCoordinatorResponse(
            Buffer.concat(chunks).toString('utf-8'),
          )
          if (res.statusCode !== 200) {
            callback(new Error(data.error ?? 'Coordinator error'))
            return
          }
          if (data.code === undefined) {
            callback(new Error('Coordinator response missing code'))
            return
          }
          const sourceMap = parseSourceMap(data.map)
          callback(null, data.code, sourceMap)
        } catch (e) {
          callback(toLoaderError(e))
        }
      })
    },
  )
  req.on('error', (err) => callback(err))
  req.write(body)
  req.end()
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
      theme,
      defaultClassMap,
      defaultFileMap,
      defaultSheet,
      importAliases = {},
    } = this.getOptions()

    // Coordinator mode: delegate to HTTP server
    if (coordinatorPortFile) {
      this.addDependency(coordinatorPortFile)
      const callback = this.async()
      const tryCoordinator = (retries: number) => {
        if (!existsSync(coordinatorPortFile)) {
          if (retries > 0) {
            setTimeout(() => tryCoordinator(retries - 1), 50)
            return
          }
          // Port file never appeared — fall through to error
          callback(new Error('Coordinator port file not found'))
          return
        }
        try {
          const port = readCoordinatorPort(coordinatorPortFile)
          // POSIX-normalize so the engine's bucket key matches the canonical map
          // and FILE_ROUTES keys (both built with forward slashes). Without this,
          // canonical collapse and atom hoisting silently no-op on Windows.
          const relativePath = relative(
            process.cwd(),
            this.resourcePath,
          ).replaceAll('\\', '/')
          const body = JSON.stringify({
            filename: relativePath,
            code: source.toString(),
            resourcePath: this.resourcePath,
          })
          coordinatorExtract(port, body, (err, content, sourceMap) => {
            if (err) return callback(err)
            callback(null, content, sourceMap as Parameters<typeof callback>[2])
          })
        } catch (error) {
          callback(toLoaderError(error))
        }
      }
      tryCoordinator(20) // 20 retries × 50ms = 1s max wait
      return
    }

    // Non-coordinator mode: local WASM extraction
    const promises: Promise<void>[] = []
    if (!init) {
      init = true
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
        if (existsSync(themeFile))
          registerTheme(
            JSON.parse(readFileSync(themeFile, 'utf-8'))?.theme ?? {},
          )
      } else {
        importFileMap(defaultFileMap)
        importClassMap(defaultClassMap)
        importSheet(defaultSheet)
        registerTheme(theme)
      }
    }

    const callback = this.async()
    try {
      const id = this.resourcePath
      let relCssDir = relative(dirname(id), cssDir).replaceAll('\\', '/')

      // POSIX-normalize (see coordinator-mode note above) so bucket keys match
      // the canonical map / FILE_ROUTES on Windows.
      const relativePath = relative(process.cwd(), id).replaceAll('\\', '/')

      if (!relCssDir.startsWith('./')) relCssDir = `./${relCssDir}`
      const { code, map, cssFile, updatedBaseStyle } = codeExtract(
        relativePath,
        source.toString(),
        libPackage,
        relCssDir,
        singleCss,
        false,
        true,
        importAliases,
      )
      const sourceMap = parseSourceMap(map)
      if (updatedBaseStyle && watch) {
        // update base style
        promises.push(
          writeFile(join(cssDir, 'devup-ui.css'), getCss(null, false), 'utf-8'),
        )
      }
      if (cssFile && watch) {
        // don't write file when build
        promises.push(
          writeFile(
            join(cssDir, basename(cssFile)),
            `/* ${this.resourcePath} ${Date.now()} */`,
          ),
          writeFile(sheetFile, exportSheet()),
          writeFile(classMapFile, exportClassMap()),
          writeFile(fileMapFile, exportFileMap()),
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
  init = false
  cachedPorts.clear()
}
