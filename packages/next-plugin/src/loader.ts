import { existsSync, readFileSync } from 'node:fs'
import { writeFile } from 'node:fs/promises'
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
  // turbo
  theme?: object
  defaultSheet: object
  defaultClassMap: object
  defaultFileMap: object
}
let init = false

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
      theme,
      defaultClassMap,
      defaultFileMap,
      defaultSheet,
    } = this.getOptions()

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
