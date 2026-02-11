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

let cachedPort: number | null = null
const keepAliveAgent = new Agent({ keepAlive: true })

function readCoordinatorPort(portFile: string): number {
  if (cachedPort !== null) return cachedPort
  cachedPort = parseInt(readFileSync(portFile, 'utf-8').trim())
  return cachedPort
}

function coordinatorExtract(
  port: number,
  body: string,
  callback: (
    err: Error | null,
    content?: string,
    sourceMap?: object | null,
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
          const data = JSON.parse(Buffer.concat(chunks).toString('utf-8'))
          if (res.statusCode !== 200) {
            callback(new Error(data.error ?? 'Coordinator error'))
            return
          }
          const sourceMap = data.map ? JSON.parse(data.map) : null
          callback(null, data.code, sourceMap)
        } catch (e) {
          callback(e instanceof Error ? e : new Error(String(e)))
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

    // Coordinator mode: delegate to HTTP server in dev mode
    if (coordinatorPortFile && watch) {
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
          const relativePath = relative(process.cwd(), this.resourcePath)
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
          callback(error as Error)
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
            JSON.parse(readFileSync(themeFile, 'utf-8'))?.['theme'] ?? {},
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

      const relativePath = relative(process.cwd(), id)

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
      const sourceMap = map ? JSON.parse(map) : null
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
            join(cssDir, basename(cssFile!)),
            `/* ${this.resourcePath} ${Date.now()} */`,
          ),
          writeFile(sheetFile, exportSheet()),
          writeFile(classMapFile, exportClassMap()),
          writeFile(fileMapFile, exportFileMap()),
        )
      }
      Promise.all(promises)
        .catch(console.error)
        .finally(() => callback(null, code, sourceMap))
    } catch (error) {
      Promise.all(promises)
        .catch(console.error)
        .finally(() => callback(error as Error))
    }
    return
  }
export default devupUILoader

/** @internal Reset init state for testing purposes only */
export const resetInit = () => {
  init = false
  cachedPort = null
}
