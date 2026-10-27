import assert from 'node:assert/strict'
import { createRequire } from 'node:module'

const { OP, withAccess } = createRequire(import.meta.url)(
  '../../../bindings/devup-ui-binding/interop.cjs',
)

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

export function runProtocol(raw, check, request) {
  for (const [name, target, input, operation, metadata, message] of [
    [
      'empty',
      raw.importCanonicalMap,
      {},
      OP.entries,
      [],
      /^Error: Array length < 2$/,
    ],
    [
      'missing-value',
      raw.importCanonicalMap,
      { file: 'canonical' },
      OP.pair,
      ['file'],
      /^Error: missing private access result$/,
    ],
    [
      'missing-origin',
      raw.importSheet,
      {},
      OP.field,
      [],
      /^Error: missing private access result$/,
    ],
    [
      'wrong-status',
      raw.importCanonicalMap,
      new Map([['file', 'canonical']]),
      OP.next,
      ['yes', undefined],
      /^Error: .*`bool`$/,
    ],
    [
      'wrong-value',
      raw.importCanonicalMap,
      {},
      OP.entries,
      [[], 'length'],
      /^Error: .*`u32`$/,
    ],
  ]) {
    check(`access-${name}`, () => {
      // Given: one malformed metadata packet; all other accesses stay authored.
      // When: native decoding consumes that packet.
      // Then: metadata conversion reports Host, not Schema.
      assert.throws(
        () =>
          withControl(
            target,
            input,
            ({ access }) =>
              (op, value, key) =>
                op === operation ? metadata : access(op, value, key),
          ),
        { name: 'Error', code: 'GenericFailure', message },
      )
    })
  }
  check('access-wrong-origin', () => {
    // Given: a Host throw whose text resembles an intrinsic Schema failure.
    const target = raw.hasDevupUi
    const marker = new Error('invalid intrinsic receiver')
    let resolverCalls = 0
    raw.setResolver(() => {
      resolverCalls++
      return undefined
    })
    try {
      // When: preflight's callback throws outside the authored access catch.
      // Then: Host propagates before selecting the resolver.
      assert.throws(
        () =>
          withControl(target, request(), ({ access }) => (op, value, key) => {
            if (op === OP.iter) throw marker
            return access(op, value, key)
          }),
        {
          name: 'Error',
          code: 'GenericFailure',
          message: /^Error: (?:Error: )?invalid intrinsic receiver$/,
        },
      )
      assert.equal(resolverCalls, 0)
    } finally {
      raw.setResolver(undefined)
    }
  })
  check('access-schema-token', () => {
    // Given: a primitive iterator receiver produces the private Schema identity.
    const target = raw.hasDevupUi
    const input = request()
    input[3] = false
    let matched = false
    // When: decode forgiving preflight through the real authored access callback.
    const result = withControl(
      target,
      input,
      ({ access, schemaToken }) =>
        (op, value, key) => {
          const result = access(op, value, key)
          if (op === OP.iter) matched = result === schemaToken
          return result
        },
    )
    // Then: only Schema defaults the aliases without a pending-failure replay.
    assert.equal(matched, true)
    assert.equal(result, false)
  })
  check('access-access-token', () => {
    // Given: an iterator lookup throw is wrapped and converted once in preflight.
    const target = raw.hasDevupUi
    const input = request()
    let conversions = 0,
      matched = false
    const marker = {
      toString() {
        conversions++
        return 'forgiven'
      },
    }
    input[3] = {
      get [Symbol.iterator]() {
        throw marker
      },
    }
    // When: the authored callback supplies the actual private Access identity.
    const result = withControl(
      target,
      input,
      ({ access, accessToken }) =>
        (op, value, key) => {
          const result = access(op, value, key)
          if (op === OP.iter) matched = result === accessToken
          return result
        },
    )
    // Then: replay forgives that import failure exactly once.
    assert.equal(matched, true)
    assert.equal(conversions, 1)
    assert.equal(result, false)
  })
  check('access-function-result', () => {
    // Given: a genuine function is a valid pair receiver, not either token.
    const target = raw.importFileMap
    const input = {}
    const pair = () => assert.fail('Function-valued pair must not be invoked')
    pair[0] = 'file'
    pair[1] = 8
    const original = Object.entries
    let matched = false
    Object.entries = (value) => (value === input ? [pair] : original(value))
    try {
      // When: the real index operation returns the function directly to native.
      withControl(target, input, ({ access }) => (op, value, key) => {
        const result = access(op, value, key)
        if (op === OP.index) matched = result === pair
        return result
      })
    } finally {
      Object.entries = original
    }
    // Then: native traverses the function pair and preserves its field values.
    assert.equal(matched, true)
    assert.deepEqual(JSON.parse(raw.exportFileMap()), { file: 8 })
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
