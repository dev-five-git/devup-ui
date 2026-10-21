const assert = require('node:assert/strict')
const { join, resolve } = require('node:path')
const fs = require('node:fs/promises')
const { put } = require('./reservation-native-fixture.cjs')
const { observe } = require('./reservation-native-observer.cjs')
const wasm = require('@devup-ui/wasm')

/** @typedef {import('./serve-coherence-async-fixture.cjs').Hooks & {load(id:string):string|undefined, configureServer(server:import('vite').ViteDevServer):void, hotUpdate(this:unknown,ctx:unknown):Promise<unknown>, handleHotUpdate(ctx:unknown):Promise<unknown>}} ProvenanceHooks */
/** @param {unknown} value @returns {value is ProvenanceHooks} */
function ProvenanceHooks(value) {
  return (
    typeof value === 'object' &&
    value !== null &&
    [
      'configResolved',
      'watchChange',
      'closeBundle',
      'buildStart',
      'transform',
      'load',
      'configureServer',
      'hotUpdate',
      'handleHotUpdate',
    ].every(
      (key) => key in value && typeof Reflect.get(value, key) === 'function',
    )
  )
}

/** @param {string} root @param {Partial<import('../plugin').DevupUIPluginOptions>} [options] */
function provenanceFixture(root, options = {}) {
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
  assert(ProvenanceHooks(plugin))
  const id = resolve(root, 'src/A.tsx').replaceAll('\\', '/')
  const context = {
    environment: { name: 'client' },
    addWatchFile(/** @type {string} */ _file) {},
  }
  const transform = (code = source, fullId = id) =>
    plugin.transform.call(context, code, fullId)
  /** @type {import('../reservation-inputs').ReservationConfig} */
  const config = { root, command: 'serve' }
  return { write, source, plugin, plugins, id, context, transform, config }
}

/** Suspend the real HTML parser result, not a fabricated reservation/revision. */
function provenanceParserGate() {
  const utils = require('@devup-ui/plugin-utils')
  const entry = require.cache[require.resolve('@devup-ui/plugin-utils')]
  assert(entry)
  const original = entry.exports
  /** @type {PromiseWithResolvers<void>} */
  const entered = Promise.withResolvers()
  /** @type {PromiseWithResolvers<void>} */
  const released = Promise.withResolvers()
  let armed = true
  entry.exports = {
    ...utils,
    async htmlReservationEntries(
      /** @type {Parameters<typeof utils.htmlReservationEntries>} */ ...args
    ) {
      const result = await utils.htmlReservationEntries(...args)
      if (armed) {
        armed = false
        entered.resolve()
        await released.promise
      }
      return result
    },
  }
  return {
    entered: entered.promise,
    release: released.resolve,
    restore() {
      released.resolve()
      entry.exports = original
    },
  }
}

/** @param {string} path */
function provenanceWriteGate(path) {
  const write = fs.writeFile
  /** @type {PromiseWithResolvers<void>} */
  const entered = Promise.withResolvers()
  /** @type {PromiseWithResolvers<void>} */
  const released = Promise.withResolvers()
  let armed = true
  fs.writeFile = new Proxy(write, {
    apply(target, receiver, args) {
      const result = Reflect.apply(target, receiver, args)
      if (!armed || String(args[0]) !== path) return result
      armed = false
      return Promise.resolve(result).then(async () => {
        entered.resolve()
        await released.promise
      })
    },
  })
  return {
    entered: entered.promise,
    release: released.resolve,
    restore() {
      released.resolve()
      fs.writeFile = write
    },
  }
}

module.exports = {
  provenanceFixture,
  provenanceParserGate,
  provenanceWriteGate,
  observe,
  wasm,
}
