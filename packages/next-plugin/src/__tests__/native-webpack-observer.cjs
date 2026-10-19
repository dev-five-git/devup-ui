const assert = require('node:assert/strict')
const { EventEmitter } = require('node:events')
const { appendFileSync } = require('node:fs')
const { join } = require('node:path')
const wasm = require('@devup-ui/wasm')
const { DevupUIWebpackPlugin } = require('@devup-ui/webpack-plugin')
const { parseClasses, parseSlots } = require('./native-webpack-receipt.ts')
const { assertReserved } = require('./native-webpack-checks.ts')

const events = new EventEmitter()
/** @type {import('./native-webpack-types.ts').NativeEvent[]} */
const records = []
let epoch = 0
let phase = 'initial'
let role = 'setup'
/** @param {import('./native-webpack-types.ts').Payload} value */
function record(value) {
  const entry = { pid: process.pid, epoch, phase, role, ...value }
  records.push(entry)
  appendFileSync(
    join(process.cwd(), 'native-evidence.jsonl'),
    `${JSON.stringify(entry)}\n`,
  )
  events.emit('record', entry)
}
function snapshot() {
  return {
    classes: parseClasses(wasm.exportClassMap()),
    files: parseSlots(wasm.exportFileMap()),
    residual: wasm.getUnreservedClassSlotCount(),
    css: wasm.getCss(null, false),
  }
}
const reset = wasm.resetBuildState
wasm.resetBuildState = (...args) => {
  record({ kind: 'reset', state: snapshot() })
  const result = reset(...args)
  epoch += 1
  return result
}
const reserve = wasm.reserveClassSlots
/** @param {import('./native-webpack-types.ts').ReserveArgs} args */
wasm.reserveClassSlots = (...args) => {
  const before = snapshot()
  const result = reserve(...args)
  record({ kind: 'reserve', args, before, after: snapshot() })
  return result
}
const extract = wasm.codeExtract
/** @param {import('./native-webpack-types.ts').ExtractArgs} args */
wasm.codeExtract = (...args) => {
  const before = wasm.getUnreservedClassSlotCount()
  record({ kind: 'extract-start', args, before })
  const result = extract(...args)
  record({
    kind: 'extract',
    args,
    before,
    after: wasm.getUnreservedClassSlotCount(),
    output: {
      code: result.code,
      css: result.css,
      cssFile: result.cssFile,
      map: result.map,
      dependencies: result.dependencies,
      updatedBaseStyle: result.updatedBaseStyle,
    },
  })
  return result
}
/**
 * @param {import('./native-webpack-types.ts').NativeConfig} config
 * @param {Parameters<NonNullable<import('next').NextConfig['webpack']>>[1]} options
 */
function configure(config, options) {
  const compilerRole = options.isServer ? options.nextRuntime : 'client'
  assert(compilerRole)
  if (options.dev && process.platform === 'win32')
    config.watchOptions = { ...config.watchOptions, poll: 100 }
  record({
    kind: 'options',
    compilerRole,
    dev: options.dev,
    buildId: options.buildId,
    delegation: config.plugins.filter(
      (plugin) => plugin instanceof DevupUIWebpackPlugin,
    ).length,
  })
  config.plugins.push({
    /** @param {import('webpack').Compiler} compiler */
    apply(compiler) {
      const select = () => {
        role = compilerRole
      }
      compiler.hooks.run.tap(
        { name: 'NativeNextEvidence', stage: -10000 },
        select,
      )
      compiler.hooks.watchRun.tap(
        { name: 'NativeNextEvidence', stage: -10000 },
        select,
      )
      compiler.hooks.thisCompilation.tap('NativeNextEvidence', () => {
        select()
        record({ kind: 'compilation', compilerName: compiler.name })
      })
      compiler.hooks.done.tap('NativeNextEvidence', (stats) => {
        select()
        record({ kind: 'done', errors: stats.hasErrors(), state: snapshot() })
      })
    },
  })
  return config
}
/**
 * @param {(records: readonly import('./native-webpack-types.ts').NativeEvent[]) => boolean} predicate
 * @param {AbortSignal} signal
 * @returns {Promise<void>}
 */
function until(predicate, signal) {
  if (predicate(records)) return Promise.resolve()
  return new Promise((resolve, reject) => {
    const deadline = AbortSignal.any([signal, AbortSignal.timeout(90000)])
    function abort() {
      events.off('record', listener)
      reject(deadline.reason)
    }
    function listener() {
      if (!predicate(records)) return
      deadline.removeEventListener('abort', abort)
      events.off('record', listener)
      resolve()
    }
    deadline.throwIfAborted()
    deadline.addEventListener('abort', abort, { once: true })
    events.on('record', listener)
  })
}
module.exports = {
  configure,
  records,
  snapshot,
  until,
  assertReserved() {
    assertReserved(records)
  },
  revise() {
    phase = 'revision'
  },
}
