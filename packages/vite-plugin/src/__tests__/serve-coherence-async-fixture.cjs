const assert = require('node:assert/strict')
const fs = require('node:fs/promises')
const { mkdtempSync, realpathSync, rmSync } = require('node:fs')
const { syncBuiltinESMExports } = require('node:module')
const { tmpdir } = require('node:os')
const { join } = require('node:path')
const { put } = require('./reservation-native-fixture.cjs')

/** @typedef {import('../reservation-inputs').ReservationConfig} Config */
/** @typedef {{configResolved(config:Config):Promise<void>, watchChange(file:string):Promise<void>, closeBundle():void|Promise<void>, buildStart(this:{addWatchFile(file:string):void},options?:{input:readonly string[]}):Promise<void>, transform(this:{addWatchFile(file:string):void},code:string,id:string):Promise<unknown>}} Hooks */
/** @param {unknown} value @returns {value is Hooks} */
function hooks(value) {
  return (
    typeof value === 'object' &&
    value !== null &&
    [
      'configResolved',
      'watchChange',
      'closeBundle',
      'buildStart',
      'transform',
    ].every(
      (key) => key in value && typeof Reflect.get(value, key) === 'function',
    )
  )
}

function project() {
  const root = realpathSync(
    mkdtempSync(join(tmpdir(), 'serve-coherence-async-')),
  )
  /** @param {string} path @param {string} text */
  const write = (path, text) => put(root, path, text)
  write('package.json', '{"name":"async-native","type":"module"}')
  write('devup.json', '{}')
  write('index.html', '<script type="module" src="/src/A.tsx"></script>')
  const source =
    'import {Box} from \'@devup-ui/react\'; export const A=()=> <Box w="$size"/>;'
  write('src/A.tsx', source)
  /** @type {{DevupUI:typeof import('../plugin').DevupUI}} */
  const { DevupUI } = require('../../dist/' + 'index.cjs')
  const plugins = DevupUI({
    devupFile: join(root, 'devup.json'),
    distDir: join(root, 'df'),
    cssDir: join(root, 'df/css'),
    singleCss: true,
  })
  assert(Array.isArray(plugins))
  const plugin = { ...plugins[0], transform: plugins[0].transform.handler }
  assert(hooks(plugin))
  /** @type {Config} */
  const config = { root, command: 'serve' }
  return {
    root,
    write,
    source,
    plugin,
    config,
    async close() {
      await plugin.closeBundle()
      rmSync(root, { recursive: true, force: true })
    },
  }
}

/** Gate one actual filesystem operation, preserving its real value and errors.
 * @param {'readFile'|'writeFile'} method @param {string} path */
function gate(method, path) {
  /** @type {PromiseWithResolvers<void>} */
  const entered = Promise.withResolvers()
  /** @type {PromiseWithResolvers<void>} */
  const release = Promise.withResolvers()
  const original = fs[method]
  let armed = true
  let calls = 0
  Object.defineProperty(fs, method, {
    value: new Proxy(original, {
      apply(target, receiver, args) {
        const result = Reflect.apply(target, receiver, args)
        if (String(args[0]) !== path) return result
        calls++
        if (!armed) return result
        armed = false
        return Promise.resolve(result).then(async (value) => {
          entered.resolve()
          await release.promise
          return value
        })
      },
    }),
  })
  syncBuiltinESMExports()
  return {
    entered: entered.promise,
    release: release.resolve,
    get calls() {
      return calls
    },
    restore() {
      release.resolve()
      Object.defineProperty(fs, method, { value: original })
      syncBuiltinESMExports()
    },
  }
}

module.exports = { project, gate, hooks }
