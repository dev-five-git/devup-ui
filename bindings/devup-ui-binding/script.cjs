'use strict'
const assert = require('node:assert/strict')
const { readFileSync, writeFileSync } = require('node:fs')
const { isAbsolute, join } = require('node:path')
const createBoundary = require('../argument-validation.cjs')
const [addon, destination] = process.argv.slice(2)
const publicMode = addon === '--public'
assert.ok((publicMode || isAbsolute(addon)) && isAbsolute(destination))
const interop = readFileSync(join(__dirname, 'interop.cjs'), 'utf8')
const adapter = readFileSync(join(__dirname, 'adapter.cjs'), 'utf8')
const boundaryImport = "require('../argument-validation.cjs')()"
const interopImport = "require('./interop.cjs')"
assert.equal(interop.split(boundaryImport).length, 2)
assert.equal(adapter.split(interopImport).length, 2)
const source = [
  "'use strict'",
  `const __devupBoundary = (${createBoundary.toString()})()`,
  'const interop = (() => {',
  'const exports = {}',
  interop.replace(boundaryImport, '__devupBoundary'),
  'return exports',
  '})()',
  'const adapter = (() => {',
  'const exports = {}',
  adapter.replace(interopImport, 'interop'),
  'return exports',
  '})()',
  ...(publicMode
    ? ["module.exports = adapter.createBinding(require('../loader.cjs'))"]
    : [
        'const native = { exports: {} }',
        `process.dlopen(native, ${JSON.stringify(addon)})`,
        'module.exports = adapter.createBinding(native.exports)',
      ]),
].join('\n')
assert.equal([...source.matchAll(/\brequire\(/g)].length, publicMode ? 1 : 0)
writeFileSync(destination, source, 'utf8')
