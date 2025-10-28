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
      singleCss,
      theme,
      defaultClassMap,
      defaultFileMap,
      defaultSheet,
    } = this.getOptions()
    const callback = this.async()
    const id = this.resourcePath
    if (!init) {
      init = true
      if (defaultFileMap) importFileMap(defaultFileMap)
      if (defaultClassMap) importClassMap(defaultClassMap)
      if (defaultSheet) importSheet(defaultSheet)
      if (theme) registerTheme(theme)
    }

    try {
      let relCssDir = relative(dirname(id), cssDir).replaceAll('\\', '/')

      const relativePath = relative(process.cwd(), id)

      if (!relCssDir.startsWith('./')) relCssDir = `./${relCssDir}`
      const {
        code,
        css = '',
        map,
        cssFile,
        updatedBaseStyle,
      } = codeExtract(
        relativePath,
        source.toString(),
        libPackage,
        relCssDir,
        singleCss,
        false,
        true,
      )
      const sourceMap = map ? JSON.parse(map) : null
      const promises: Promise<void>[] = []
      if (updatedBaseStyle) {
        // update base style
        promises.push(
          writeFile(join(cssDir, 'devup-ui.css'), getCss(null, false), 'utf-8'),
        )
      }
      if (cssFile) {
        const content = `${this.resourcePath} ${Date.now()}`
        // should be reset css
        promises.push(
          writeFile(
            join(cssDir, basename(cssFile!)),
            watch ? `/* ${content} */` : css,
          ),
        )
        if (watch) {
          promises.push(
            writeFile(sheetFile, exportSheet()),
            writeFile(classMapFile, exportClassMap()),
            writeFile(fileMapFile, exportFileMap()),
          )
        }
        Promise.all(promises)
          .catch(console.error)
          .finally(() => callback(null, code, sourceMap))
        return
      }
      callback(null, code, sourceMap)
    } catch (error) {
      callback(error as Error)
    }
    return
  }
export default devupUILoader
