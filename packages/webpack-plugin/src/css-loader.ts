import { getFileNumByFilename } from '@devup-ui/plugin-utils'
import { getCss } from '@devup-ui/wasm'
import type { RawLoaderDefinitionFunction } from 'webpack'

import { servedCss } from './served-css'

const devupUICssLoader: RawLoaderDefinitionFunction = function (_, map, meta) {
  // The stylesheet comes from every extracted module, not from this file
  this.cacheable(false)
  const css = getCss(getFileNumByFilename(this.resourcePath), true)
  if (this._compilation)
    servedCss(this._compilation).set(this.resourcePath, css)
  this.callback(null, css, map, meta)
}
export default devupUICssLoader
