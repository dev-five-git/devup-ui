import assert from 'node:assert/strict'

import { pkg } from './corpus/extraction.mjs'
import {
  barrel,
  diagnostic,
  expectedObject,
  rejected,
  source,
} from './entries-resolver-fixtures.mjs'
import { runResolverPublication } from './entries-resolver-publication.mjs'

export async function runResolverContracts(h) {
  const { w } = h
  const malformed = [
    ['number', () => 7, expectedObject],
    ['string', () => 'private-looking cause', expectedObject],
    [
      'function',
      () => Object.assign(() => {}, { path: 'x', code: '' }),
      expectedObject,
    ],
    ['array', () => Object.assign([], { path: 'x', code: '' }), expectedObject],
    ['missing-path', () => ({}), '`path` must be a string'],
    ['number-path', () => ({ path: 4 }), '`path` must be a string'],
    [
      'empty-path',
      () => ({
        path: '',
        get code() {
          throw new Error('empty path must stop before code')
        },
      }),
      '`path` must be a non-empty string',
    ],
    ['missing-code', () => ({ path: 'x' }), '`code` must be a string'],
    ['number-code', () => ({ path: 'x', code: 4 }), '`code` must be a string'],
  ]
  for (const [name, resolver, cause] of malformed)
    await h.test(`resolver-strict-${name}`, () => {
      // Given
      w.setModuleResolver(resolver)
      // When
      const error = rejected(h, () =>
        w.hasDevupUI('src/strict.tsx', barrel, pkg, {}),
      )
      // Then
      assert.equal(
        error,
        diagnostic(
          'src/strict.tsx',
          1,
          19,
          './barrel',
          'src/strict.tsx',
          `malformed module resolver result: ${cause}`,
        ),
      )
      return error
    })
  for (const [name, value, cause] of [
    ['string', 'broken Ω', 'broken Ω'],
    ['error', new Error('broken Ω'), 'broken Ω'],
    ['null', null, 'JsValue(null)'],
    ['undefined', undefined, 'JsValue(undefined)'],
    ['message', { message: 'message Ω' }, 'message Ω'],
    ['nonstring-message', { message: 4 }, 'JsValue(Object({"message":4}))'],
    [
      'throwing-message',
      {
        get message() {
          throw null
        },
      },
      'JsValue(Object)',
    ],
    [
      'no-string-coercion',
      {
        toString() {
          assert.fail('must not coerce cause')
        },
      },
      'JsValue(Object({}))',
    ],
  ])
    for (const phase of ['callback', 'path', 'code'])
      await h.test(`resolver-cause-${name}-${phase}`, () => {
        // Given
        const events = []
        w.setModuleResolver(function () {
          assert.equal(this, null)
          events.push('callback')
          if (phase === 'callback') throw value
          return {
            get path() {
              events.push('path')
              if (phase === 'path') throw value
              return 'x'
            },
            get code() {
              events.push('code')
              throw value
            },
          }
        })
        // When
        const error = rejected(h, () =>
          w.hasDevupUI('src/cause.tsx', barrel, pkg, {}),
        )
        // Then
        assert.equal(
          error,
          diagnostic(
            'src/cause.tsx',
            1,
            19,
            './barrel',
            'src/cause.tsx',
            cause,
          ),
        )
        assert.deepEqual(events, [
          'callback',
          ...(phase === 'callback' ? [] : ['path']),
          ...(phase === 'code' ? ['code'] : []),
        ])
        return { error, events }
      })
  for (const value of [null, undefined])
    await h.test(`resolver-nullish-${value}`, () => {
      // Given
      w.setModuleResolver(() => value)
      // When
      const eligible = w.hasDevupUI('src/nullish.tsx', barrel, pkg, {})
      const output = h.read(h.open({ code: source }))
      // Then
      assert.equal(eligible, false)
      return { eligible, output }
    })
  await h.test('resolver-transitive-repeated-original-sites', () => {
    // Given
    const code = `import {Box} from './barrel';\nexport * from './barrel';\nexport const v=<Box p={2}/>`
    let failures = 0
    w.setModuleResolver((specifier, importer) => {
      if (importer === 'src/transitive.tsx')
        return {
          path: 'src/barrel.ts',
          code: `export {Box} from './leaf';\nexport * from './leaf';`,
        }
      assert.equal(specifier, './leaf')
      failures++
      throw 'leaf failure'
    })
    // When
    const error = rejected(h, () =>
      h.open({ file: 'src/transitive.tsx', code }),
    )
    const repeated = rejected(h, () =>
      h.open({ file: 'src/transitive.tsx', code }),
    )
    // Then
    assert.equal(repeated, error)
    assert.ok(
      failures > 1,
      'exercise repeated failed resolution, not just two sites',
    )
    assert.equal(
      error,
      [
        diagnostic(
          'src/barrel.ts',
          1,
          19,
          './leaf',
          'src/barrel.ts',
          'leaf failure',
        ),
        diagnostic(
          'src/barrel.ts',
          2,
          15,
          './leaf',
          'src/barrel.ts',
          'leaf failure',
        ),
      ].join('\n'),
    )
    return { error, failures }
  })
  await runResolverPublication(h)
}
