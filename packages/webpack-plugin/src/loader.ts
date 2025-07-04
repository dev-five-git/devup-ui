import { writeFile } from 'node:fs/promises'
import { dirname, relative } from 'node:path'

import { codeExtract, exportClassMap, exportSheet } from '@devup-ui/wasm'
import type { RawLoaderDefinitionFunction } from 'webpack'

export interface DevupUILoaderOptions {
  package: string
  cssFile: string
  sheetFile: string
  classMapFile: string
  watch: boolean
}

const devupUILoader: RawLoaderDefinitionFunction<DevupUILoaderOptions> =
  function (source) {
    const {
      watch,
      package: libPackage,
      cssFile,
      sheetFile,
      classMapFile,
    } = this.getOptions()
    const callback = this.async()
    const id = this.resourcePath

    try {
      const { code, css, map } = codeExtract(
        id,
        source.toString(),
        libPackage,
        relative(dirname(this.resourcePath), cssFile).replaceAll('\\', '/'),
      )
      if (css && watch) {
        const content = `${this.resourcePath} ${Date.now()}`
        if (this._compiler) (this._compiler as any).__DEVUP_CACHE = content
        // should be reset css
        Promise.all([
          writeFile(cssFile, `/* ${content} */`),
          writeFile(sheetFile, exportSheet()),
          writeFile(classMapFile, exportClassMap()),
        ])
          .catch(console.error)
          .finally(() => callback(null, code, map))
        return
      }
      callback(null, code, map)
    } catch (error) {
      callback(error as Error)
    }
    return
  }
export default devupUILoader
