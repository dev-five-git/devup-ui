import { resolve } from 'node:path'

import type { RawLoaderDefinitionFunction } from 'webpack'

const devupUICssLoader: RawLoaderDefinitionFunction = function (a) {
  this.addContextDependency(resolve(this.rootContext, 'src'))
  this.callback(null, a)
}
export default devupUICssLoader
