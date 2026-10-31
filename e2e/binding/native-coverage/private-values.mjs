import assert from 'node:assert/strict'
import { createRequire } from 'node:module'

const { OP, withAccess } = createRequire(import.meta.url)(
  '../../../bindings/devup-ui-binding/interop.cjs',
)

export function privateError(message, code) {
  return (error) => {
    assert.equal(error.constructor, Error)
    assert.equal(Object.getPrototypeOf(error), Error.prototype)
    assert.equal(error.name, 'Error')
    assert.equal(error.message, message)
    assert.equal(Object.hasOwn(error, 'code'), code !== undefined)
    if (code !== undefined) assert.equal(error.code, code)
    else assert.equal(Object.hasOwn(error, 'cause'), false)
    return true
  }
}

export function withControl(target, input, intercept) {
  return withAccess(
    (value, access, schemaToken, accessToken) =>
      target(
        value,
        intercept({ access, schemaToken, accessToken }),
        schemaToken,
        accessToken,
      ),
    input,
  )
}

export function runMapStart(raw, check, request) {
  const iterator = { next: () => ({ done: true }) }
  const callable = () => assert.fail('Metadata payload must not be invoked')
  const coercible = {
    valueOf: () => assert.fail('Metadata must not be coerced'),
  }
  const arrayError = privateError('Failed to get Array length', 'ArrayExpected')
  const missingError = privateError('Error: missing private access result')
  const startError = privateError('Error: invalid private map start')
  const iteratorError = privateError('Error: invalid private map iterator')
  const tagError = (type) =>
    privateError(
      `Failed to convert napi value ${type} into rust type \`f64\``,
      'NumberExpected',
    )
  const lengthError = privateError(
    'Failed to convert napi value String into rust type `u32`',
    'NumberExpected',
  )
  const malformed = [
    [undefined, arrayError],
    [null, arrayError],
    [{}, arrayError],
    [callable, arrayError],
    [[], missingError],
    [['1', iterator], tagError('String')],
    [[undefined], tagError('Undefined')],
    [[3], startError],
    [[0.5], startError],
    [[4294967296], startError],
    [[NaN], startError],
    [[Infinity], startError],
    [[Object(1), iterator], tagError('Object')],
    [[coercible], tagError('Object')],
    [[0, 'extra'], startError],
    [[1], startError],
    [[1, iterator, 'extra'], startError],
    [[2, []], startError],
    [[2, [], 0, 'extra'], startError],
    [[1, undefined], iteratorError],
    [[1, null], iteratorError],
    [[1, callable], iteratorError],
    [[2, [], 'length'], lengthError],
  ]
  for (const [index, [packet, expected]] of malformed.entries()) {
    check(`map-envelope-${index}`, () => {
      // Given: malformed metadata at the genuine addon's map-start boundary.
      let resolver = 0,
        schema = 0,
        forgiveness = 0
      raw.setResolver(() => {
        resolver++
        return undefined
      })
      const intercept =
        ({ access }) =>
        (op, value, key) => {
          if (op === OP.schema) schema++
          if (op === OP.preflightFailure) forgiveness++
          return op === OP.mapStart ? packet : access(op, value, key)
        }
      try {
        // When: strict and preflight decode the same invalid envelope.
        for (const [target, input] of [
          [raw.importCanonicalMap, {}],
          [raw.hasDevupUi, request()],
        ])
          assert.throws(() => withControl(target, input, intercept), expected)
        // Then: Host/Transport never notify Schema, forgive, or select the resolver.
        assert.deepEqual([resolver, schema, forgiveness], [0, 0, 0])
      } finally {
        raw.setResolver(undefined)
      }
    })
  }
  check('map-start-initial-next-once', () => {
    // Given: a real iterator with an observable initial next getter.
    let reads = 0
    const source = {
      [Symbol.iterator]: () => ({
        get next() {
          reads++
          return () => ({ done: true })
        },
      }),
    }
    // When: the genuine addon completes an empty map iteration.
    withControl(raw.importCanonicalMap, source, ({ access }) => access)
    // Then: one startup read plus one Next read, with no envelope revalidation.
    assert.equal(reads, 2)
  })
  check('map-start-unrestricted-empty-entries', () => {
    for (const entries of [undefined, null, 3, callable]) {
      withControl(
        raw.importCanonicalMap,
        {},
        ({ access }) =>
          (op, value, key) =>
            op === OP.mapStart ? [2, entries, 0] : access(op, value, key),
      )
    }
  })
  check('map-start-nonfunction-fallback-raw', () => {
    // Given: a rejected method and a raw entries failure, including nullish throws.
    const original = Object.entries
    for (const marker of [undefined, null, coercible]) {
      const source = { [Symbol.iterator]: 3 }
      Object.entries = (value) => {
        if (value === source) throw marker
        return original(value)
      }
      try {
        // When/Then: fallback preserves the original thrown identity without String.
        let caught = false
        try {
          withControl(raw.importCanonicalMap, source, ({ access }) => access)
        } catch (error) {
          caught = true
          assert.equal(error, marker)
        }
        assert.equal(caught, true)
      } finally {
        Object.entries = original
      }
    }
  })
}

export function runValues(w, check, file) {
  const sheet = (s, extra = {}) => ({
    properties: {
      [file]: {
        0: { 0: [{ c: 'private', p: 'color', v: 'red', s, ...extra }] },
      },
    },
  })
  for (const kind of ['Media', { Media: null }, { Media: undefined }]) {
    check(`unit-enum-${JSON.stringify(kind)}`, () => {
      w.importSheet(
        sheet(
          { At: { kind, query: '(width > 1px)', selector: null } },
          { t: true, h: false },
        ),
      )
      assert.match(w.getCss(undefined, false), /red/)
    })
  }
  for (const s of [
    true,
    1,
    null,
    new Uint8Array([1]),
    { At: { kind: { Media: 1 }, query: '' } },
  ]) {
    check(`malformed-selector-${String(s)}`, () => {
      // Null is a valid absent selector; malformed inner unit payload is not.
      if (s === null) w.importSheet(sheet(s))
      else assert.throws(() => w.importSheet(sheet(s)), TypeError)
    })
  }
  for (const value of [
    1.5,
    1n,
    18446744073709551615n,
    18446744073709551616n,
    -9223372036854775809n,
    new Uint8Array([1]),
    new ArrayBuffer(1),
    new Set([1]),
    Symbol('color'),
  ]) {
    check(`malformed-color-${String(value)}`, () => {
      assert.throws(
        () => w.registerTheme({ colors: { default: { probe: value } } }),
        TypeError,
      )
    })
  }
  check('non-map-iterable-color', () => {
    const value = { [Symbol.iterator]: () => [][Symbol.iterator]() }
    assert.throws(
      () => w.registerTheme({ colors: { default: { probe: value } } }),
      TypeError,
    )
  })
  check('typed-string-rejects-bytes', () => {
    assert.throws(
      () => w.importCanonicalMap({ file: new Uint8Array([1]) }),
      TypeError,
    )
  })
  check('bool-schema-and-map-array', () => {
    assert.throws(() => w.importSheet(sheet(null, { t: 1 })), TypeError)
    w.importFileRoutes(new Map([['a', [1, 2]]]))
    w.importClassMap({ a: { zero: 0n, max: 4294967295n } })
    assert.deepEqual(JSON.parse(w.exportClassMap()), {
      a: { zero: 0, max: 4294967295 },
    })
    for (const value of [18446744073709551616n, -1n, 4294967296n])
      assert.throws(
        () => w.importClassMap({ a: { invalid: value } }),
        TypeError,
      )
  })
}
