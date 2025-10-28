import { getCss } from '@devup-ui/wasm'
import type { RawLoaderDefinitionFunction } from 'webpack'

function getFileNumByFilename(filename: string) {
  if (filename.endsWith('devup-ui.css')) return null
  return parseInt(filename.split('devup-ui-')[1].split('.')[0])
}

const devupUICssLoader: RawLoaderDefinitionFunction = function (_, map, meta) {
  const fileNum = getFileNumByFilename(this.resourcePath)
  this.callback(null, getCss(fileNum, true), map, meta)
}
export default devupUICssLoader
