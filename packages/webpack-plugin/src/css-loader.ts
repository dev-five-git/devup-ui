import { getCss } from '@devup-ui/wasm'
import type { RawLoaderDefinitionFunction } from 'webpack'

const devupUICssLoader: RawLoaderDefinitionFunction = function () {
  this.callback(null, getCss())
}
export default devupUICssLoader
