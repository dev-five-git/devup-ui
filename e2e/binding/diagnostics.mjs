import assert from 'node:assert/strict'
import { inspect } from 'node:util'

const creation = new Set([
  'stack',
  'originalLine',
  'originalColumn',
  'line',
  'column',
  'sourceURL',
])
export const rawDiagnostics = []
const inspection = {
  showHidden: true,
  depth: null,
  maxStringLength: null,
  maxArrayLength: null,
}

function field(value, ancestors) {
  if (value instanceof Error) return diagnostic(value, ancestors)
  switch (typeof value) {
    case 'undefined':
      return { type: 'undefined' }
    case 'string':
    case 'boolean':
      return { type: typeof value, value }
    case 'number':
      return {
        type: 'number',
        value: Object.is(value, -0) ? '-0' : String(value),
      }
    case 'bigint':
      return { type: 'bigint', value: String(value) }
    case 'object':
      if (value === null) return { type: 'null' }
      break
  }
  assert.fail(`Unsupported diagnostic field category: ${typeof value}`)
}

function propertyKey(key) {
  if (typeof key === 'string') return { type: 'string', value: key }
  const registered = Symbol.keyFor(key)
  if (registered !== undefined)
    return { type: 'registered-symbol', value: registered }
  const known = Object.getOwnPropertyNames(Symbol).find(
    (name) => Symbol[name] === key,
  )
  assert.ok(known, 'Unsupported local-symbol diagnostic key')
  return { type: 'well-known-symbol', value: known }
}

export function diagnostic(error, ancestors = new Set()) {
  assert.ok(error instanceof Error)
  assert.ok(!ancestors.has(error), 'Unsupported cyclic diagnostic cause')
  const next = new Set(ancestors).add(error)
  const prototypes = []
  for (let p = Object.getPrototypeOf(error); p; p = Object.getPrototypeOf(p))
    prototypes.push(p.constructor.name)
  const properties = Reflect.ownKeys(error).map((key) => {
    const descriptor = Object.getOwnPropertyDescriptor(error, key)
    const flags = {
      enumerable: descriptor.enumerable,
      configurable: descriptor.configurable,
    }
    if ('value' in descriptor) flags.writable = descriptor.writable
    else {
      assert.equal(key, 'stack', 'Unsupported diagnostic accessor')
      flags.get = typeof descriptor.get
      flags.set = typeof descriptor.set
    }
    const value = 'value' in descriptor ? descriptor.value : error[key]
    let prefix
    if (key === 'stack') {
      assert.equal(typeof value, 'string')
      const lines = value.split('\n')
      const frame = lines.findIndex((line) =>
        /^\s*at |^[^@]*@.*:\d+/.test(line),
      )
      prefix = lines.slice(0, frame < 0 ? lines.length : frame).join('\n')
    }
    return {
      key: propertyKey(key),
      flags,
      value: creation.has(key)
        ? {
            type: typeof value,
            ...(key === 'stack' ? { prefix } : {}),
          }
        : field(value, next),
    }
  })
  rawDiagnostics.push({
    inspection: inspect(error, inspection),
    descriptors: inspect(Object.getOwnPropertyDescriptors(error), inspection),
  })
  return { constructor: error.constructor.name, prototypes, properties }
}

export function runDiagnostics(w) {
  // Given: controls differing only in a hidden value or a descriptor flag.
  for (const change of [{ value: 2 }, { writable: true }]) {
    const a = new Error('control'),
      b = new Error('control')
    Object.defineProperty(a, Symbol.for('diagnostic-control'), { value: 1 })
    Object.defineProperty(b, Symbol.for('diagnostic-control'), {
      value: 1,
      ...change,
    })
    // When / Then: the exact diagnostic comparison must reject either mismatch.
    assert.notDeepEqual(diagnostic(a), diagnostic(b))
  }
  const nested = new Error('outer', { cause: new Error('inner') })
  const other = new Error('outer', { cause: new Error('inner') })
  Object.defineProperty(other.cause, 'hidden', { value: true })
  assert.notDeepEqual(diagnostic(nested), diagnostic(other))
  const records = []
  const check = (name, action, Constructor = Error) => {
    // Given: one public failing call in an isolated engine state.
    w.resetStateForTesting()
    let caught = false
    try {
      // When: invoke the public API, retaining the actual thrown object.
      action()
    } catch (error) {
      caught = true
      // Then: compare all fields against the realm's ordinary error shape.
      assert.equal(error.constructor, Constructor, name)
      const observed = diagnostic(error)
      assert.deepEqual(
        observed,
        diagnostic(new Constructor(error.message)),
        name,
      )
      records.push({ name, diagnostic: observed })
    }
    assert.ok(caught, `${name} must throw`)
  }
  const pkg = '@devup-ui/react'
  const open = (method, code, aliases = {}) =>
    w[method]('diagnostic.tsx', code, pkg, 'df', true, false, false, aliases)
  const imported = `import { Box } from '${pkg}'; import { color } from './tokens'; export const view=<Box color={color}/>;`
  for (const method of ['codeExtract', 'codeExtractWithoutSourceMap']) {
    check(`${method}-syntax`, () =>
      open(method, `import {Box} from '${pkg}'; const x=<Box p={}/>;`),
    )
    check(`${method}-extraction`, () =>
      open(
        method,
        `import {css} from '${pkg}'; export const x=css({color:window.name});`,
      ),
    )
    for (const result of [
      7,
      { path: 7 },
      { path: '' },
      { path: 'tokens.ts', code: 7 },
    ])
      check(`${method}-resolver-${JSON.stringify(result)}`, () => {
        w.setModuleResolver(() => result)
        open(method, imported)
      })
    check(`${method}-resolver-exception`, () => {
      const original = new Error('resolver marker', {
        cause: new Error('user cause'),
      })
      w.setModuleResolver(() => {
        throw original
      })
      try {
        open(method, imported)
      } catch (error) {
        assert.notEqual(error, original)
        assert.equal(Object.hasOwn(error, 'cause'), false)
        throw error
      }
    })
  }
  check('preflight-resolver', () => {
    w.setModuleResolver(() => {
      throw new Error('resolver marker')
    })
    w.hasDevupUI(
      'diagnostic.tsx',
      "import {Box} from './barrel'; const x=<Box/>;",
      pkg,
      {},
    )
  })
  check('shorthand-target', () => w.registerShorthands({ invalid: ['widht'] }))
  check('argument-validation', () => w.setPrefix(false), TypeError)
  check('iterator-import', () =>
    w.importFileMap({
      [Symbol.iterator]() {
        throw new Error('iterator marker')
      },
    }),
  )
  const cause = new Error('retained cause')
  const original = Object.assign(new Error('raw marker', { cause }), {
    code: 'user-code',
  })
  Object.defineProperty(original, Symbol.for('user-hidden'), { value: 7 })
  assert.throws(
    () =>
      w.importFileMap({
        get a() {
          throw original
        },
      }),
    (error) => {
      assert.equal(error, original)
      assert.equal(error.cause, cause)
      records.push({ name: 'raw-user-identity', diagnostic: diagnostic(error) })
      return true
    },
  )
  for (const state of ['shell', 'released']) {
    for (const key of [
      'code',
      'map',
      'css',
      'cssFile',
      'updatedBaseStyle',
      'dependencies',
      'free',
      Symbol.dispose,
    ]) {
      check(`Output-${state}-${String(key)}`, () => {
        const output =
          state === 'shell'
            ? new w.Output()
            : open('codeExtract', 'export const x=1;')
        if (state === 'released') output.free()
        return key === 'free' || typeof key === 'symbol'
          ? output[key]()
          : output[key]
      })
    }
  }
  w.resetStateForTesting()
  return records
}
