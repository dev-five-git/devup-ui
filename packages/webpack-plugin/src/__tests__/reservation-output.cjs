const assert = require('node:assert/strict')
const { readFileSync } = require('node:fs')
const { join } = require('node:path')
const wasm = require('@devup-ui/wasm')
const { state } = require('./reservation-loaders.cjs')

module.exports = function snapshot(stats, fixture, extracts) {
  assert(!stats.hasErrors(), stats.toString({ all: false, errors: true }))
  const files = JSON.parse(wasm.exportFileMap())
  const css = [
    wasm.getCss(null, false),
    ...[...new Set(Object.values(files))].map((number) =>
      wasm.getCss(number, false),
    ),
  ].join('\n')
  assert(!css.includes('magenta'), 'unused scratch CSS must not leak')
  return {
    transformed: { ...state.transformed },
    classes: JSON.parse(wasm.exportClassMap()),
    files,
    canonical: JSON.parse(wasm.exportCanonicalMap()),
    metadata: Object.fromEntries(
      extracts
        .filter((item) => !item.prewarm)
        .map((item) => [
          item.id,
          { map: item.map, dependencies: item.dependencies },
        ]),
    ),
    css,
    assets: Object.fromEntries(
      Object.keys(stats.compilation.assets)
        .sort()
        .map((name) => [
          name,
          readFileSync(join(fixture, 'out', name), 'utf8'),
        ]),
    ),
    residual: wasm.getUnreservedClassSlotCount(),
  }
}
