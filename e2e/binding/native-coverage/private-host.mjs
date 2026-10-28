import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFileSync, realpathSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, isAbsolute, join } from 'node:path'

import { runProtocol } from './private-protocol.mjs'
import { runValues, withControl } from './private-values.mjs'

const [entry, destination, group] = process.argv.slice(2)
assert.equal(process.argv.length, 5)
assert.ok(isAbsolute(entry) && isAbsolute(destination))
assert.equal(process.versions.bun, undefined)
assert.ok(['protocol', 'raw-resolver-failure'].includes(group))
const require = createRequire(import.meta.url)
const image = realpathSync(join(dirname(entry), 'binding.node'))
const raw = require(image)
const {
  OP,
  withAccess,
} = require('../../../bindings/devup-ui-binding/interop.cjs')
const {
  createBinding,
} = require('../../../bindings/devup-ui-binding/adapter.cjs')
const records = []
const check = (name, action) => {
  action()
  records.push(name)
}
const file = 'native-private.tsx'
const pkg = '@devup-ui/react'
const code =
  'import { Box } from "./barrel"; export const x = <Box color="red" />'
const request = () => [file, code, pkg, {}]
const call = (method, value) => withAccess(raw[method], value)

check('state-before-init', () => {
  assert.throws(() => raw.reset(false), /native facade is not initialized/)
  assert.throws(
    () => raw.setResolver(undefined),
    /native facade is not initialized/,
  )
})
const w = createBinding(raw)
check('repeat-init-retains-resolver', () => {
  let calls = 0
  w.setModuleResolver(() => {
    calls++
    return { path: 'barrel.tsx', code: 'export { Box } from "@devup-ui/react"' }
  })
  raw.init()
  assert.equal(w.hasDevupUI(...request()), true)
  assert.ok(calls > 0)
  w.resetBuildState()
  assert.equal(w.hasDevupUI(...request()), false)
})

if (group === 'raw-resolver-failure') {
  check('unwrapped-resolver-throw', () => {
    const marker = new Error('private raw resolver throw')
    let calls = 0
    raw.setResolver(() => {
      calls++
      throw marker
    })
    assert.throws(
      () => call('hasDevupUi', request()),
      (error) => error === marker,
    )
    assert.ok(calls > 0)
  })
  check('unwrapped-resolver-invalid-resolved', () => {
    let calls = 0
    raw.setResolver(() => {
      calls++
      return { path: 3, code: '' }
    })
    assert.throws(() => call('hasDevupUi', request()), /Resolved.path/)
    assert.ok(calls > 0)
  })
} else {
  for (const method of ['hasDevupUi', 'extract', 'reserve']) {
    check(`${method}-short-request`, () => {
      assert.throws(() => call(method, []), /missing private request field/)
    })
    check(`${method}-wrong-request`, () => {
      assert.throws(
        () => call(method, method === 'reserve' ? [[], false] : [false]),
        /String/,
      )
    })
  }
  check('interface-name-arity', () => {
    for (const names of [[], ['a'], ['a', 'b', 'c', 'd', 'e', 'f']])
      assert.throws(
        () => raw.getThemeInterface(pkg, names),
        /invalid private interface-name request/,
      )
    assert.throws(() => raw.getThemeInterface(pkg, [false]), /String/)
  })
  check('schema-callback-throw', () => {
    const marker = new Error('private schema callback throw')
    const target = raw.importCanonicalMap
    assert.throws(
      () =>
        withControl(target, false, ({ access }) => (op, value, key) => {
          if (op === OP.schema) throw marker
          return access(op, value, key)
        }),
      (error) => error === marker,
    )
  })
  check('preflight-host-failure', () => {
    const target = raw.hasDevupUi
    assert.throws(
      () =>
        withControl(
          target,
          request(),
          ({ access }) =>
            (op, value, key) =>
              op === OP.mapStart ? [] : access(op, value, key),
        ),
      {
        name: 'Error',
        code: 'GenericFailure',
        message: /^Error: missing private access result$/,
      },
    )
  })
  check('preflight-access-failure', () => {
    const marker = new Error('private preflight access throw')
    // Iterator getter failures are intentionally converted and forgiven. A raw
    // entry getter is not an iterator-import failure and must escape unchanged.
    const aliases = {
      get broken() {
        throw marker
      },
    }
    assert.throws(
      () => call('hasDevupUi', [file, code, pkg, aliases]),
      (error) => error === marker,
    )
  })
  check('preflight-failure-operation-host', () => {
    const target = raw.hasDevupUi
    assert.throws(
      () =>
        withControl(
          target,
          request(),
          ({ access, accessToken }) =>
            (op, value, key) => {
              if (op === OP.mapStart) return accessToken
              if (op === OP.preflightFailure)
                throw new Error('private preflight failure host')
              return access(op, value, key)
            },
        ),
      {
        name: 'Error',
        code: 'GenericFailure',
        message: /^Error: (?:Error: )?private preflight failure host$/,
      },
    )
  })
  check('reservation-row-arity', () => {
    assert.throws(
      () => w.reserveClassSlots([['file.tsx']], pkg, false, false, false, {}),
      TypeError,
    )
  })
  check('unknown-field-is-not-read', () => {
    let reads = 0
    const operations = []
    const target = raw.importSheet
    withControl(
      target,
      {
        get unknown() {
          reads++
          throw Error('unknown field read')
        },
      },
      ({ access }) =>
        (op, value, key) => {
          operations.push([op, key])
          return access(op, value, key)
        },
    )
    assert.equal(reads, 0)
    assert.deepEqual(
      operations,
      [
        'atom_plan',
        'properties',
        'css',
        'keyframes',
        'global_css_files',
        'imports',
        'font_faces',
        'layer_orders',
      ].map((key) => [OP.field, key]),
    )
  })
  runValues(w, check, file)
  runProtocol(raw, check, request)
}
w.resetStateForTesting()
writeFileSync(
  join(destination, 'complete.json'),
  JSON.stringify(
    {
      group,
      entry: realpathSync(entry),
      image,
      node: process.execPath,
      version: process.version,
      sha256: createHash('sha256').update(readFileSync(image)).digest('hex'),
      records,
    },
    null,
    2,
  ) + '\n',
  { flag: 'wx' },
)
console.info(JSON.stringify({ group, passed: records.length }))
