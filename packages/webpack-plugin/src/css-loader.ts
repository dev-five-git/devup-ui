import { getCss } from '@devup-ui/wasm'
import type { RawLoaderDefinitionFunction } from 'webpack'

let prevData = ''
let prevTime = ''

const devupUICssLoader: RawLoaderDefinitionFunction<{
  watch: boolean
}> = function (source) {
  const { watch } = this.getOptions()
  if (!watch) return this.callback(null, getCss())
  const stringSource =
    (this._compiler as any)?.__DEVUP_CACHE || source.toString()

  if (prevTime === stringSource) {
    this.callback(null, prevData)
    return
  }
  prevTime = stringSource
  this.callback(null, (prevData = getCss()))
}
export default devupUICssLoader
