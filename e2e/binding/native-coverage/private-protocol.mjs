import assert from 'node:assert/strict'
import { createRequire } from 'node:module'

const { OP } = createRequire(import.meta.url)(
  '../../../bindings/devup-ui-binding/interop.cjs',
)

import { privateError, runMapStart, withControl } from './private-values.mjs'

export function runProtocol(raw, check, request) {
  runMapStart(raw, check, request)
  for (const [name, target, input, operation, metadata, expected] of [
    [
      'empty',
      raw.importCanonicalMap,
      {},
      OP.mapStart,
      [],
      privateError('Error: missing private access result'),
    ],
    [
      'missing-value',
      raw.importCanonicalMap,
      { file: 'canonical' },
      OP.pair,
      ['file'],
      privateError('Error: missing private access result'),
    ],
    [
      'missing-origin',
      raw.importSheet,
      {},
      OP.field,
      [],
      privateError('Error: missing private access result'),
    ],
    [
      'wrong-status',
      raw.importCanonicalMap,
      new Map([['file', 'canonical']]),
      OP.next,
      ['yes', undefined],
      privateError(
        'Failed to convert napi value into rust type `bool`',
        'BooleanExpected',
      ),
    ],
    [
      'wrong-value',
      raw.importCanonicalMap,
      {},
      OP.mapStart,
      [2, [], 'length'],
      privateError(
        'Failed to convert napi value String into rust type `u32`',
        'NumberExpected',
      ),
    ],
  ]) {
    check(`access-${name}`, () => {
      // Given: one malformed metadata packet; all other accesses stay authored.
      // When: native decoding consumes that packet.
      // Then: retain the binding diagnostic or exact transport status, not Schema.
      assert.throws(
        () =>
          withControl(
            target,
            input,
            ({ access }) =>
              (op, value, key) =>
                op === operation ? metadata : access(op, value, key),
          ),
        expected,
      )
    })
  }
  check('access-wrong-origin', () => {
    // Given: a raw throw whose text resembles an intrinsic Schema failure.
    const target = raw.hasDevupUi
    const marker = new Error('invalid intrinsic receiver')
    let resolverCalls = 0
    raw.setResolver(() => {
      resolverCalls++
      return undefined
    })
    try {
      // When: preflight's callback throws outside the authored access catch.
      // Then: the original Error propagates before selecting the resolver.
      assert.throws(
        () =>
          withControl(target, request(), ({ access }) => (op, value, key) => {
            if (op === OP.mapStart) throw marker
            return access(op, value, key)
          }),
        (error) => error === marker,
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
          if (op === OP.mapStart) matched = result === schemaToken
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
          if (op === OP.mapStart) matched = result === accessToken
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
