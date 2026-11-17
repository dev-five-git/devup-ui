import { existsSync, readFileSync } from 'node:fs'

import {
  type DevupCompiler,
  type DevupEngine,
  getFileNumByFilename,
} from '@devup-ui/plugin-utils'
import type { RawLoaderDefinitionFunction } from 'webpack'

import { callCoordinator, resetCoordinatorClient } from './coordinator-client'
import { loadWasm } from './wasm'

export interface DevupUICssLoaderOptions {
  engine?: DevupEngine
  // turbo
  watch: boolean
  coordinatorPortFile?: string
  sheetFile: string
  classMapFile: string
  fileMapFile: string
  themeFile: string
  /** Every file the theme is read from (devup.json, its extends, Tailwind CSS) */
  themeFiles?: string[]
  theme?: object
  defaultSheet: object
  defaultClassMap: object
  defaultFileMap: object
}

let initializedCompilers = new WeakSet<DevupCompiler>()
function fetchCssFromCoordinator(
  portFile: string,
  fileNum: number | null,
  importMainCss: boolean,
  waitForIdle: boolean,
  callback: (err: Error | null, css?: string) => void,
): void {
  const params = new URLSearchParams()
  if (fileNum != null) params.set('fileNum', String(fileNum))
  params.set('importMainCss', String(importMainCss))
  if (waitForIdle) params.set('waitForIdle', 'true')
  callCoordinator(
    portFile,
    { path: `/css?${params.toString()}`, method: 'GET' },
    (error, reply) => {
      if (reply === undefined) {
        callback(error)
        return
      }
      if (reply.statusCode !== 200) {
        callback(new Error(`Coordinator CSS error: ${reply.statusCode}`))
        return
      }
      callback(null, reply.body)
    },
  )
}

const devupUICssLoader: RawLoaderDefinitionFunction<DevupUICssLoaderOptions> =
  function (source, map, meta) {
    const {
      watch,
      engine,
      coordinatorPortFile,
      sheetFile,
      classMapFile,
      fileMapFile,
      themeFile,
      themeFiles = [],
      theme,
      defaultClassMap,
      defaultFileMap,
      defaultSheet,
    } = this.getOptions()

    // Coordinator mode: fetch live CSS from coordinator (disk content is stale)
    if (coordinatorPortFile) {
      // The sheet holds the theme's variables: re-run when the theme changes.
      for (const file of themeFiles) this.addDependency(file)
      const callback = this.async()
      // Check both resourcePath and resourceQuery for ?fileNum=N
      // Turbopack may embed query in resourcePath or provide it via resourceQuery
      const resourceQuery =
        (this as unknown as { resourceQuery?: string }).resourceQuery ?? ''
      const pathWithQuery = this.resourcePath + resourceQuery
      const fileNum = getFileNumByFilename(pathWithQuery)
      const importMainCss = fileNum !== null
      fetchCssFromCoordinator(
        coordinatorPortFile,
        fileNum,
        importMainCss,
        !watch,
        callback,
      )
      return
    }

    let content: string | typeof source
    try {
      const compiler = loadWasm(engine)
      const {
        getCss,
        importClassMap,
        importFileMap,
        importSheet,
        registerTheme,
      } = compiler
      if (!initializedCompilers.has(compiler)) {
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
        initializedCompilers.add(compiler)
      }

      content = !watch
        ? source
        : getCss(getFileNumByFilename(this.resourcePath), true)
    } catch (error) {
      this.callback(error instanceof Error ? error : new Error(String(error)))
      return
    }
    this.callback(null, content, map, meta)
  }
export default devupUICssLoader

/** @internal Reset init state for testing purposes only */
export const resetInit = () => {
  initializedCompilers = new WeakSet<DevupCompiler>()
  resetCoordinatorClient()
}

export { setWasmForTesting } from './wasm'
