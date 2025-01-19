import { writeFileSync } from 'node:fs'

import { codeExtract } from '@devup-ui/wasm'
import type { RawLoaderDefinitionFunction } from 'webpack'

import { type DevupUIWebpackPlugin } from './plugin'

export interface DevupUILoaderOptions {
  plugin: DevupUIWebpackPlugin
}

const devupUILoader: RawLoaderDefinitionFunction<DevupUILoaderOptions> =
  function (source) {
    const { plugin } = this.getOptions()
    const { package: libPackage, cssFile } = plugin.options
    const callback = this.async()
    const id = this.resourcePath
    if (
      id.includes('node_modules/') ||
      id.includes('@devup-ui/react') ||
      !/\.[tj](s|sx)?$/.test(id)
    ) {
      callback(null, source)
      return
    }
    this.addDependency(cssFile)

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
        })
      }

      callback(null, code)
    } catch (error) {
      callback(error as Error)
    }
    return
  }
export default devupUILoader
