import { writeFileSync } from 'node:fs'

import { codeExtract } from '@devup-ui/wasm'
import type { RawLoaderDefinitionFunction } from 'webpack'

interface DevupUiLoaderOptions {
  package: string
  cssFile: string
}

const devupUiLoader: RawLoaderDefinitionFunction<DevupUiLoaderOptions> =
  function (source) {
    const { package: libPackage, cssFile } = this.getOptions()
    const callback = this.async()
    const id = this.resourcePath
    if (
      id.includes('/node_modules/') ||
      id.includes('@devup-ui/react') ||
      !/\.[tj](s|sx)?$/.test(id)
    ) {
      callback(null, source)
      return
    }
    try {
      const { code, css } = codeExtract(
        this.resourcePath,
        source.toString(),
        libPackage,
        cssFile,
      )
      if (css) {
        // should be reset css
        writeFileSync(cssFile, css, {
          encoding: 'utf-8',
          flag: 'a',
        })
      }

      callback(null, code)
    } catch (error) {
      callback(error as Error)
    }
    return
  }
export default devupUiLoader
