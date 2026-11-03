import {
  type DevupEngine,
  getFileNumByFilename,
  loadCompiler,
} from '@devup-ui/plugin-utils'
import type { RawLoaderDefinitionFunction } from 'webpack'

import { servedCss } from './served-css'

export interface DevupUICssLoaderOptions {
  readonly engine?: DevupEngine
  readonly watch?: boolean
}

const devupUICssLoader: RawLoaderDefinitionFunction<DevupUICssLoaderOptions> =
  function (_, map, meta) {
    // The stylesheet comes from every extracted module, not from this file
    this.cacheable(false)
    const compiler = loadCompiler(this.getOptions().engine)
    const css = compiler.getCss(getFileNumByFilename(this.resourcePath), true)
    if (this._compilation)
      servedCss(this._compilation).set(this.resourcePath, css)
    this.callback(null, css, map, meta)
  }
export default devupUICssLoader
