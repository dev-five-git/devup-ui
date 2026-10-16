const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const { basename, join } = require('node:path')
const wasm = require('@devup-ui/wasm')
const { getFileNumByFilename } = require('@devup-ui/plugin-utils')

let generation = 0
let baseBuilt = false
const pending = []
let sequence = 0
function record(event, data) {
  if (process.env.DEVUP_WATCH_RECEIPT)
    fs.appendFileSync(
      process.env.DEVUP_WATCH_RECEIPT,
      JSON.stringify(
        { sequence: ++sequence, generation, event, ...data },
        (_, value) => (value === undefined ? { undefined: true } : value),
      ) + '\n',
    )
}
function engine() {
  const files = wasm.exportFileMap()
  return {
    sheet: wasm.exportSheet(),
    classes: wasm.exportClassMap(),
    files,
    css: [null, ...new Set(Object.values(JSON.parse(files)))].map((number) => [
      number,
      wasm.getCss(number, true),
    ]),
    residual:
      typeof wasm.getUnreservedClassSlotCount === 'function'
        ? wasm.getUnreservedClassSlotCount()
        : null,
  }
}
const extract = wasm.codeExtract
wasm.codeExtract = (...args) => {
  const result = extract(...args)
  record('extract', {
    args,
    output: Object.fromEntries(
      ['code', 'css', 'map', 'dependencies', 'cssFile', 'updatedBaseStyle'].map(
        (key) => [key, result[key]],
      ),
    ),
    engine: engine(),
  })
  return result
}
const load = Module._load
Module._load = function (request, parent, isMain) {
  const result = load.call(this, request, parent, isMain)
  if (request.endsWith('reservation-loaders.cjs')) {
    const gate = function (source) {
      if (!baseBuilt && /^[AB]\.css\.ts$/.test(basename(this.resourcePath))) {
        const callback = this.async()
        record('hold-layer', {
          resource: this.resourcePath,
          source: source.toString(),
        })
        pending.push(() => callback(null, source))
        return
      }
      return result.call(this, source)
    }
    return Object.assign(gate, result)
  }
  if (!request.endsWith('/compiled/webpack/webpack')) return result
  return {
    ...result,
    webpack(options) {
      options.plugins.push({
        apply(compiler) {
          compiler.hooks.thisCompilation.tap(
            'WatchGenerationObserver',
            (compilation) => {
              generation++
              baseBuilt = false
              compilation.hooks.succeedModule.tap(
                'WatchGenerationObserver',
                (module) => {
                  if (module.resource?.endsWith('.css')) {
                    record('css-module', {
                      resource: module.resource,
                      source: module.originalSource()?.source().toString(),
                      engine: engine(),
                    })
                    if (basename(module.resource) === 'devup-ui.css') {
                      baseBuilt = true
                      for (const release of pending.splice(0)) release()
                    }
                  }
                },
              )
            },
          )
          compiler.hooks.done.tap('WatchGenerationObserver', (stats) => {
            const served = [
              ...(globalThis.__devupUiServedCss?.get(stats.compilation) ?? []),
            ]
            assert(
              served.length > 0,
              'native generation must build CSS modules',
            )
            const current = served.map(([path]) => [
              path,
              wasm.getCss(getFileNumByFilename(path), true),
            ])
            record('done', {
              served,
              current,
              engine: engine(),
              errors: stats.hasErrors(),
              assets: Object.fromEntries(
                Object.keys(stats.compilation.assets).map((name) => [
                  name,
                  fs.readFileSync(
                    join(compiler.options.output.path, name),
                    'utf8',
                  ),
                ]),
              ),
            })
            assert.deepEqual(
              served,
              current,
              'every delivered watch generation must contain exact current CSS, including late layers',
            )
          })
        },
      })
      return result.webpack(options)
    },
  }
}
