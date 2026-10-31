import assert from 'node:assert/strict'

import { pkg, source } from './corpus/extraction.mjs'
import { indexed, reserve, runAliasHostile, slot } from './entries-fixtures.mjs'

export async function runSlots(h) {
  const { w } = h
  await h.test('root-slots-precedence-nested-schema', () => {
    const before = h.snapshot()
    const poison = {
      get [Symbol.iterator]() {
        return assert.fail('Primitive validation must precede aliases')
      },
    }
    h.schema(
      () => w.codeExtract(4, source, pkg, 'df', false, false, false, poison),
      slot.filename,
    )
    const cases = [
      [() => w.importClassMap({ a: { color: -1 } }), slot.classes],
      [
        () =>
          w.importFileMap({
            [Symbol.iterator]: function* () {
              yield null
            },
          }),
        slot.file,
      ],
      [() => reserve(w, [['bad']], poison), slot.rows],
      [() => reserve(w, [], { bad: 4 }), slot.aliases],
    ]
    for (const [action, message] of cases) h.schema(action, message)
    for (const mode of ['codeExtract', 'codeExtractWithoutSourceMap'])
      h.schema(
        () => h.open({ mode, aliases: { bad: 4 } }),
        `${mode}(): argument import_aliases must be an alias mapping`,
      )
    assert.deepEqual(
      h.snapshot(),
      before,
      'Rejected root slots must not mutate core state',
    )
    return cases.map(([, message]) => message)
  })
  await h.test('aliases-forgiving-raw-and-later-error', () => {
    const events = [],
      sentinel = {},
      aliases = {
        get a() {
          events.push('get')
          throw sentinel
        },
      }
    h.raw(() => w.hasDevupUI('src/ok.tsx', source, pkg, aliases), sentinel)
    h.raw(() => h.open({ aliases }), sentinel)
    h.raw(() => reserve(w, [], aliases), sentinel)
    for (const value of [undefined, null, { bad: 7 }])
      assert.equal(w.hasDevupUI('src/ok.tsx', source, pkg, value), true)
    const later = h.take(() =>
      w.hasDevupUI(
        'src/later.tsx',
        "import { Box } from '@devup-ui/react'; const =",
        pkg,
        aliases,
      ),
    )
    assert.equal(later.caught, true)
    assert.equal(later.error, sentinel)
    assert.deepEqual(events, ['get', 'get', 'get', 'get'])
    return { events, laterCaught: later.caught, laterValue: later.value }
  })
  await h.test('aliases-strict-nullish-diagnostic-and-access', () => {
    const events = [],
      sentinel = {
        [Symbol.toPrimitive]() {
          assert.fail('Access replay must remain raw')
        },
      }
    for (const mode of ['codeExtract', 'codeExtractWithoutSourceMap']) {
      for (const aliases of [null, undefined])
        h.schema(
          () => h.open({ mode, aliases }),
          `${mode}(): argument import_aliases must be an alias mapping`,
        )
      h.raw(
        () =>
          h.open({
            mode,
            aliases: {
              get a() {
                events.push(mode)
                throw sentinel
              },
            },
          }),
        sentinel,
      )
    }
    for (const aliases of [null, undefined])
      h.schema(() => reserve(w, [], aliases), slot.aliases)
    const diagnostic = {
      bad: {
        toJSON() {
          events.push('diagnostic')
          return 7
        },
      },
    }
    assert.equal(
      w.hasDevupUI('src/diagnostic.tsx', source, pkg, diagnostic),
      true,
    )
    assert.deepEqual(events, [
      'codeExtract',
      'codeExtractWithoutSourceMap',
      'diagnostic',
    ])
    return events
  })
  await runAliasHostile(h)
  await h.test('aliases-schema-whole-map-diagnostics', () => {
    const events = [],
      code = "import Box from '@alias'; export const v=<Box/>"
    const check = (aliases) => w.hasDevupUI('src/whole.tsx', code, pkg, aliases)
    assert.equal(check({ '@alias': 'Box' }), true)
    assert.equal(check({}), false)
    for (const aliases of [
      undefined,
      null,
      { '@alias': 'Box', bad: 7 },
      new Map([
        ['@alias', 'Box'],
        ['bad', 7],
      ]),
      {
        '@alias': 'Box',
        bad: {
          toJSON() {
            events.push('toJSON')
            throw null
          },
        },
      },
      {
        [Symbol.iterator]: function* () {
          yield ['@alias', 'Box']
          throw {
            toString() {
              events.push('String')
              return 'partial'
            },
          }
        },
      },
    ])
      assert.equal(check(aliases), false)
    assert.deepEqual(events, ['toJSON', 'String'])
    return events
  })
  for (const phase of ['entries', 'enumeration', 'length'])
    await h.test(`reentry-${phase}-outer-frame`, () => {
      const events = [],
        inner = {},
        outer = {},
        target = {}
      const nested = () => {
        h.schema(() => w.importClassMap({ a: { color: -1 } }), slot.classes)
        h.raw(
          () =>
            w.importFileMap({
              get a() {
                throw inner
              },
            }),
          inner,
        )
        events.push('nested-schema-access')
      }
      const original = Object.entries
      const result = indexed((_value, key) => {
        assert.equal(key, 'length')
        nested()
        throw outer
      })
      h.raw(
        () =>
          h.patch(
            {
              value(value) {
                if (value !== target)
                  return Reflect.apply(original, this, [value])
                if (phase === 'entries') {
                  nested()
                  throw outer
                }
                if (phase === 'enumeration')
                  return original({
                    get a() {
                      nested()
                      throw outer
                    },
                  })
                return result
              },
            },
            () => w.importFileMap(target),
          ),
        outer,
      )
      assert.deepEqual(events, ['nested-schema-access'])
      const aliases = {
        get bad() {
          nested()
          return 4
        },
      }
      h.schema(() => h.open({ aliases }), slot.extraction)
      return events
    })
}
