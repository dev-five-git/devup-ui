import { getCss } from '@devup-ui/wasm'
import type { RawLoaderDefinitionFunction } from 'webpack'

let prevData = ''
let prevTime = ''

function getFileNumByFilename(filename: string) {
  if (filename.endsWith('devup-ui.css')) return null
  return parseInt(filename.split('devup-ui-')[1].split('.')[0])
}

const devupUICssLoader: RawLoaderDefinitionFunction<{
  watch: boolean
}> = function (source, map, meta) {
  const { watch } = this.getOptions()
  const fileNum = getFileNumByFilename(this.resourcePath)
  if (!watch) return this.callback(null, getCss(fileNum))
  const stringSource =
    (this._compiler as any)?.__DEVUP_CACHE || source.toString()

  if (prevTime === stringSource) {
    this.callback(null, prevData, map, meta)
    return
  }
  prevTime = stringSource
  this.callback(null, (prevData = getCss(fileNum)), map, meta)
}
export default devupUICssLoader
