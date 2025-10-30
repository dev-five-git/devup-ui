import { writeFile } from 'node:fs/promises'
import { basename, dirname, join, relative } from 'node:path'

import {
  codeExtract,
  exportClassMap,
  exportFileMap,
  exportSheet,
  getCss,
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
      singleCss,
    } = this.getOptions()
    const callback = this.async()
    const id = this.resourcePath

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
      }
      Promise.all(promises)
        .catch(console.error)
        .finally(() => callback(null, code, sourceMap))
    } catch (error) {
      callback(error as Error)
    }
    return
  }
export default devupUILoader
