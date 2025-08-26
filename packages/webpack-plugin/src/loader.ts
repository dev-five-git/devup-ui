import { writeFile } from 'node:fs/promises'
import { basename, dirname, join, relative } from 'node:path'

import { codeExtract, exportClassMap, exportSheet } from '@devup-ui/wasm'
import type { RawLoaderDefinitionFunction } from 'webpack'

export interface DevupUILoaderOptions {
  package: string
  cssDir: string
  sheetFile: string
  classMapFile: string
  watch: boolean
  splitCss: boolean
}

const devupUILoader: RawLoaderDefinitionFunction<DevupUILoaderOptions> =
  function (source) {
    const {
      watch,
      package: libPackage,
      cssDir,
      sheetFile,
      classMapFile,
      splitCss,
    } = this.getOptions()
    const callback = this.async()
    const id = this.resourcePath

    try {
      let rel = relative(dirname(this.resourcePath), cssDir).replaceAll(
        '\\',
        '/',
      )
      if (!rel.startsWith('./')) rel = `./${rel}`
      const { code, css, map, css_file } = codeExtract(
        id,
        source.toString(),
        libPackage,
        rel,
        splitCss,
      )
      const sourceMap = map ? JSON.parse(map) : null
      if (css) {
        const content = `${this.resourcePath} ${Date.now()}`
        if (watch && this._compiler)
          (this._compiler as any).__DEVUP_CACHE = content
        // should be reset css
        Promise.all([
          writeFile(
            join(cssDir, basename(css_file)),
            watch ? `/* ${content} */` : css,
          ),
          watch ? writeFile(sheetFile, exportSheet()) : null,
          watch ? writeFile(classMapFile, exportClassMap()) : null,
        ])
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
