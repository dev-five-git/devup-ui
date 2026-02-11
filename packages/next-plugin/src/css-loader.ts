import { existsSync, readFileSync } from 'node:fs'
import { Agent, request } from 'node:http'

import {
  getCss,
  importClassMap,
  importFileMap,
  importSheet,
  registerTheme,
} from '@devup-ui/wasm'
import type { RawLoaderDefinitionFunction } from 'webpack'

function getFileNumByFilename(filename: string) {
  // Handle query parameter format: devup-ui.css?fileNum=79
  // Turbopack may embed query params in resourcePath
  const queryMatch = filename.match(/[?&]fileNum=(\d+)/)
  if (queryMatch) return parseInt(queryMatch[1])
  if (filename.endsWith('devup-ui.css')) return null
  return parseInt(filename.split('devup-ui-')[1].split('.')[0])
}

export interface DevupUICssLoaderOptions {
  // turbo
  watch: boolean
  coordinatorPortFile?: string
  sheetFile: string
  classMapFile: string
  fileMapFile: string
  themeFile: string
  theme?: object
  defaultSheet: object
  defaultClassMap: object
  defaultFileMap: object
}

let init = false
let cachedPort: number | null = null
const keepAliveAgent = new Agent({ keepAlive: true })

function readCoordinatorPort(portFile: string): number {
  if (cachedPort !== null) return cachedPort
  cachedPort = parseInt(readFileSync(portFile, 'utf-8').trim())
  return cachedPort
}

function fetchCssFromCoordinator(
  port: number,
  fileNum: number | null,
  importMainCss: boolean,
  waitForIdle: boolean,
  callback: (err: Error | null, css?: string) => void,
): void {
  const params = new URLSearchParams()
  if (fileNum != null) params.set('fileNum', String(fileNum))
  params.set('importMainCss', String(importMainCss))
  if (waitForIdle) params.set('waitForIdle', 'true')
  const req = request(
    {
      hostname: '127.0.0.1',
      port,
      path: `/css?${params.toString()}`,
      method: 'GET',
      agent: keepAliveAgent,
    },
    (res) => {
      const chunks: Buffer[] = []
      res.on('data', (chunk: Buffer) => chunks.push(chunk))
      res.on('end', () => {
        if (res.statusCode !== 200) {
          callback(new Error(`Coordinator CSS error: ${res.statusCode}`))
          return
        }
        callback(null, Buffer.concat(chunks).toString('utf-8'))
      })
    },
  )
  req.on('error', (err) => callback(err))
  req.end()
}

const devupUICssLoader: RawLoaderDefinitionFunction<DevupUICssLoaderOptions> =
  function (source, map, meta) {
    const {
      watch,
      coordinatorPortFile,
      sheetFile,
      classMapFile,
      fileMapFile,
      themeFile,
      theme,
      defaultClassMap,
      defaultFileMap,
      defaultSheet,
    } = this.getOptions()

    // Coordinator mode: fetch live CSS from coordinator (disk content is stale)
    if (coordinatorPortFile) {
      const callback = this.async()
      // Check both resourcePath and resourceQuery for ?fileNum=N
      // Turbopack may embed query in resourcePath or provide it via resourceQuery
      const resourceQuery =
        (this as unknown as { resourceQuery?: string }).resourceQuery ?? ''
      const pathWithQuery = this.resourcePath + resourceQuery
      const fileNum = getFileNumByFilename(pathWithQuery)
      const importMainCss = fileNum !== null
      const tryFetch = (retries: number) => {
        if (!existsSync(coordinatorPortFile)) {
          if (retries > 0) {
            setTimeout(() => tryFetch(retries - 1), 50)
            return
          }
          callback(new Error('Coordinator port file not found'))
          return
        }
        try {
          const port = readCoordinatorPort(coordinatorPortFile)
          fetchCssFromCoordinator(
            port,
            fileNum,
            importMainCss,
            !watch,
            (err, css) => {
              if (err) return callback(err)
              callback(null, css)
            },
          )
        } catch (error) {
          callback(error as Error)
        }
      }
      tryFetch(20)
      return
    }

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

    this.callback(
      null,
      !watch ? source : getCss(getFileNumByFilename(this.resourcePath), true),
      map,
      meta,
    )
  }
export default devupUICssLoader

/** @internal Reset init state for testing purposes only */
export const resetInit = () => {
  init = false
  cachedPort = null
}
