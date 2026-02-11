import { getFileNumByFilename } from '@devup-ui/plugin-utils'
import { getCss } from '@devup-ui/wasm'
import type { RawLoaderDefinitionFunction } from 'webpack'

const devupUICssLoader: RawLoaderDefinitionFunction = function (_, map, meta) {
  const fileNum = getFileNumByFilename(this.resourcePath)
  this.callback(null, getCss(fileNum, true), map, meta)
}
export default devupUICssLoader
