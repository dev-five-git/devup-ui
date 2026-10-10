import { existsSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import {
  collectDevupConfigFiles,
  getFileNumByFilename,
} from '@devup-ui/plugin-utils'
import type { RawLoaderDefinitionFunction } from 'webpack'

import {
  CoordinatorRequestError,
  requestCoordinator,
} from './coordinator-client'
import type { CoordinatorIdentity } from './coordinator-port'
import { loadWasm } from './wasm'

export interface DevupUICssLoaderOptions {
  watch: boolean
  coordinatorPortFile?: string
  coordinatorIdentity?: CoordinatorIdentity
  projectRoot?: string
  revisionFile?: string
  themeFiles?: string[]
  requestTimeoutMs?: number
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

const devupUICssLoader: RawLoaderDefinitionFunction<DevupUICssLoaderOptions> =
  function (source, map, meta) {
    const {
      watch,
      coordinatorPortFile,
      coordinatorIdentity,
      projectRoot = process.cwd(),
      revisionFile,
      themeFiles,
      requestTimeoutMs,
      sheetFile,
      classMapFile,
      fileMapFile,
      themeFile,
      theme,
      defaultClassMap,
      defaultFileMap,
      defaultSheet,
    } = this.getOptions()
    if (coordinatorPortFile) {
      const callback = this.async()
      const fileNum = getFileNumByFilename(
        this.resourcePath + (this.resourceQuery ?? ''),
      )
      const params = new URLSearchParams({
        importMainCss: String(fileNum !== null),
      })
      if (fileNum !== null) params.set('fileNum', String(fileNum))
      if (!watch) params.set('waitForIdle', 'true')
      const operation = {
        portFile: coordinatorPortFile,
        identity: coordinatorIdentity,
        resourcePath: this.resourcePath,
        path: `/css?${params}`,
        timeoutMs: requestTimeoutMs,
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
      requestCoordinator(operation).then(
        (css) => callback(null, css),
        (error: Error) => callback(error),
      )
      return
    }
    const {
      getCss,
      importClassMap,
      importFileMap,
      importSheet,
      registerTheme,
    } = loadWasm()
    if (!init) {
      if (watch) {
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
    this.callback(
      null,
      !watch ? source : getCss(getFileNumByFilename(this.resourcePath), true),
      map,
      meta,
    )
  }
export default devupUICssLoader

export const resetInit = () => {
  init = false
}
export { setWasmForTesting } from './wasm'
