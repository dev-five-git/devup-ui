const assert = require('node:assert/strict')
const { join, resolve } = require('node:path')
const { put } = require('./reservation-native-fixture.cjs')
const { hooks, gate } = require('./serve-coherence-async-fixture.cjs')
const { observe } = require('./reservation-native-observer.cjs')
const wasm = require('@devup-ui/wasm')

/** @typedef {import('./serve-coherence-async-fixture.cjs').Hooks & {load(id:string):string|undefined}} CacheHooks */
/** @param {unknown} value @returns {value is CacheHooks} */
function cacheHooks(value) {
  return hooks(value) && 'load' in value && typeof value.load === 'function'
}
/** @param {string} root @param {Partial<import('../plugin').DevupUIPluginOptions>} [options] */
function fixture(root, options = {}) {
  const write = (/** @type {string} */ path, /** @type {string} */ text) =>
    put(root, path, text)
  write('devup.json', '{}')
  write('package.json', '{"type":"module"}')
  const source =
    "import {Box} from '@devup-ui/react'; export const A=()=> <Box color='red'/>;"
  write('src/A.tsx', source)
  const { DevupUI } = require('../../dist/' + 'index.cjs')
  const plugins = DevupUI({
    singleCss: true,
    devupFile: join(root, 'devup.json'),
    distDir: join(root, 'df'),
    cssDir: join(root, 'df/css'),
    ...options,
  })
  assert(Array.isArray(plugins))
  const plugin = plugins[0]
  assert(cacheHooks(plugin))
  const id = resolve(root, 'src/A.tsx').replaceAll('\\', '/')
  /** @type {string[]} */
  const watches = []
  const context = {
    environment: { name: 'client' },
    addWatchFile(/** @type {string} */ file) {
      watches.push(file)
    },
  }
  const transform = (code = source, fullId = id) =>
    plugin.transform.call(context, code, fullId)
  /** @type {import('../reservation-inputs').ReservationConfig} */
  const config = { root, command: 'serve' }
  return {
    root,
    write,
    source,
    plugin,
    id,
    context,
    watches,
    transform,
    config,
  }
}
module.exports = { fixture, gate, observe, wasm }
