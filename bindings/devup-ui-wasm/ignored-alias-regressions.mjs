import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { createModuleResolver } from '../../packages/plugin-utils/dist/index.mjs'
import { verifyIgnoredSemantics } from './ignored-semantics-regressions.mjs'

const wasm = createRequire(import.meta.url)('./pkg/index.js')
const filename = '/src/ignored.tsx'
const root = mkdtempSync(join(tmpdir(), 'devup-ignored-'))
const file = (name, code) => {
  const path = join(root, name)
  writeFileSync(path, code)
  return path
}
const barrel = file(
  'barrel.js',
  "export {default as def, missing as value} from 'empty'; export * as ns from 'empty'; export * from 'empty';",
)
const cjs = file(
  'cjs.js',
  "const ns = require('empty'); const {missing} = require('empty'); exports.value = missing; exports.other = ns.absent;",
)
const whole = file('whole.js', "module.exports = require('empty');")
const real = file('real.js', 'module.exports = {};')
const shared = createModuleResolver({
  cwd: root,
  alias: { empty: false, barrel, cjs, whole, real },
})
try {
  for (const method of ['codeExtract', 'codeExtractWithoutSourceMap']) {
    verifyIgnoredSemantics(wasm, method, shared)
    wasm.resetBuildState()
    wasm.setModuleResolver(() => ({ ignored: true }))
    const output = wasm[method](
      filename,
      "import { Box } from '@devup-ui/react'; import { missing } from 'empty'; export const view = <Box color={missing} bg='red' />",
      '@devup-ui/react',
      'df',
      true,
      false,
      false,
      {},
    )
    assert.deepEqual(output.dependencies, [])
    assert.doesNotMatch(output.code, /--/)
    assert.doesNotMatch(wasm.getCss(undefined, false), /color:/)
    output.free()
    for (const binding of [
      "import * as ns from 'empty'; const value = ns.missing;",
      "import def from 'empty'; const value = def.missing;",
      "import {value} from 'barrel';",
      "import {ns} from 'barrel'; const value = ns.missing;",
      "import {value} from 'cjs';",
      "import {other as value} from 'cjs';",
      "import def from 'whole'; const value = def.missing;",
    ]) {
      wasm.resetBuildState()
      wasm.setModuleResolver(shared)
      const resolved = wasm[method](
        filename,
        `import {Box} from '@devup-ui/react'; ${binding} export const view = <Box color={value} bg='red'/>;`,
        '@devup-ui/react',
        'df',
        true,
        false,
        false,
        {},
      )
      assert.doesNotMatch(resolved.code, /--/)
      assert.doesNotMatch(wasm.getCss(undefined, false), /color:/)
      if (method === 'codeExtractWithoutSourceMap')
        assert.equal(resolved.map, undefined)
      else assert.equal(typeof resolved.map, 'string')
      resolved.free()
    }
    wasm.resetBuildState()
    wasm.setModuleResolver(shared)
    const evaluated = wasm[method](
      '/src/ignored.css.ts',
      "import {style} from '@devup-ui/react'; import def, {missing} from 'empty'; import * as ns from 'empty'; import {ns as indirect, def as indirectDefault, value} from 'barrel'; function color() { if (missing !== undefined || value !== undefined || Object.keys(ns).length || Object.keys(def).length || Object.keys(indirect).length || Object.keys(indirectDefault).length) throw new Error('bad interop'); return 'red'; } export const cls = style({color: color()});",
      '@devup-ui/react',
      'df',
      true,
      false,
      false,
      {},
    )
    assert.match(wasm.getCss(undefined, false), /color:red/)
    assert.deepEqual(evaluated.dependencies, [barrel])
    evaluated.free()

    wasm.resetBuildState()
    const ignored = {
      ignored: true,
      get path() {
        throw new Error('must not read path')
      },
      get code() {
        throw new Error('must not read code')
      },
      get sourceType() {
        throw new Error('must not read sourceType')
      },
    }
    wasm.setModuleResolver(() => ignored)
    const bypassed = wasm[method](
      filename,
      "import {Box} from '@devup-ui/react'; import {missing} from 'empty'; export const view = <Box color={missing}/>;",
      '@devup-ui/react',
      'df',
      true,
      false,
      false,
      {},
    )
    assert.deepEqual(bypassed.dependencies, [])
    bypassed.free()

    for (const result of [
      true,
      false,
      1,
      'ignored',
      { ignored: false },
      { ignored: null },
      { ignored: 1 },
      { ignored: 'true' },
      {
        get ignored() {
          throw new Error('ignored getter first')
        },
      },
      new Proxy(
        {},
        {
          get() {
            throw new Error('ignored proxy first')
          },
        },
      ),
    ]) {
      wasm.resetBuildState()
      wasm.setModuleResolver(undefined)
      const initial = wasm[method](
        filename,
        "import {Box} from '@devup-ui/react'; export const view = <Box bg='blue'/>;",
        '@devup-ui/react',
        'df',
        true,
        false,
        false,
        {},
      )
      initial.free()
      const sheet = wasm.exportSheet()
      let calls = 0
      wasm.setModuleResolver(() => {
        calls += 1
        return result
      })
      assert.throws(
        () =>
          wasm[method](
            filename,
            "import {Box} from '@devup-ui/react'; import {missing} from 'empty'; import {other} from 'later'; export const view = <Box color={missing} bg={other}/>;",
            '@devup-ui/react',
            'df',
            true,
            false,
            false,
            {},
          ),
        /module resolver cannot use `empty` at build time/,
      )
      assert.equal(calls, 1)
      assert.equal(wasm.exportSheet(), sheet)
    }

    wasm.resetBuildState()
    wasm.setModuleResolver(shared)
    const previous = wasm[method](
      filename,
      "import {Box} from '@devup-ui/react'; export const view = <Box bg='blue'/>;",
      '@devup-ui/react',
      'df',
      true,
      false,
      false,
      {},
    )
    previous.free()
    const sheet = wasm.exportSheet()
    assert.throws(
      () =>
        wasm[method](
          filename,
          "import {css} from '@devup-ui/react'; import def from 'empty'; def.color = 'red'; export const cls = css(def);",
          '@devup-ui/react',
          'df',
          true,
          false,
          false,
          {},
        ),
      /changed/,
    )
    assert.equal(wasm.exportSheet(), sheet)
  }
  mkdirSync(join(root, 'src'))
  const request = 'private-next-instrumentation-client-user'
  const next = createModuleResolver({
    cwd: root,
    alias: {
      [request]: [
        join(root, 'src/instrumentation-client'),
        join(root, 'instrumentation-client'),
        'private-next-empty-module',
      ],
      'private-next-empty-module': false,
    },
  })
  assert.deepEqual(next(request, join(root, 'entry.js')), { ignored: true })
  const instrumentation = file(
    'instrumentation-client.js',
    "export const value = 'blue';",
  )
  assert.equal(next(request, join(root, 'entry.js')).path, instrumentation)
} finally {
  wasm.setModuleResolver(undefined)
  rmSync(root, { recursive: true, force: true })
}
console.info('Ignored alias actual WASM regressions passed.')
