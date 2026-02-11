import { existsSync, readFileSync } from 'node:fs'

import {
  getCss,
  importClassMap,
  importFileMap,
  importSheet,
  registerTheme,
} from '@devup-ui/wasm'
import type { RawLoaderDefinitionFunction } from 'webpack'

function getFileNumByFilename(filename: string) {
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

    // Coordinator mode: CSS already written by coordinator, pass through as-is
    if (coordinatorPortFile && watch) {
      this.callback(null, source, map, meta)
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
}
