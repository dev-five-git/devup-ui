import { getCss } from '@devup-ui/wasm'
import type { RawLoaderDefinitionFunction } from 'webpack'

function getFileNumByFilename(filename: string) {
  if (filename.endsWith('devup-ui.css')) return null
  return parseInt(filename.split('devup-ui-')[1].split('.')[0])
}

export interface DevupUICssLoaderOptions {
  // turbo
  theme: object
  defaultSheet: object
  defaultClassMap: object
  defaultFileMap: object
  watch: boolean
}

const devupUICssLoader: RawLoaderDefinitionFunction<DevupUICssLoaderOptions> =
  function (source, map, meta) {
    const { watch } = this.getOptions()
    this.callback(
      null,
      !watch ? source : getCss(getFileNumByFilename(this.resourcePath), true),
      map,
      meta,
    )
  }
export default devupUICssLoader
