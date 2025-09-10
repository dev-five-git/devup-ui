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
      let rel = relative(dirname(this.resourcePath), cssDir).replaceAll(
        '\\',
        '/',
      )

      if (!rel.startsWith('./')) rel = `./${rel}`
      const { code, css, map, cssFile, updatedBaseStyle } = codeExtract(
        id,
        source.toString(),
        libPackage,
        rel,
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
      if (css) {
        const content = `${this.resourcePath} ${Date.now()}`
        if (watch && this._compiler)
          (this._compiler as any).__DEVUP_CACHE = content
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
