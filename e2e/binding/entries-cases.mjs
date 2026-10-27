import assert from 'node:assert/strict'

import { box } from './corpus/extraction.mjs'
import { indexed, slot } from './entries-fixtures.mjs'

export async function runCases(h) {
  const { w } = h
  await h.test('maps-modes-state-nested', () => {
    const outputs = []
    for (const mode of ['codeExtract', 'codeExtractWithoutSourceMap']) {
      for (const aliases of [
        {},
        { '@alias': 'Box' },
        new Map([['@alias', 'Box']]),
      ]) {
        const out = h.open({
          mode,
          aliases,
          code: "import Box from '@alias'; export const v=<Box p={2}/>",
        })
        outputs.push(h.read(out))
      }
    }
    for (const [name, value] of [
      ['importFileMap', {}],
      ['importFileMap', { 'src/a.tsx': 4 }],
      ['importCanonicalMap', {}],
      ['importCanonicalMap', { 'src/a.tsx': 'src/root.tsx' }],
      ['importClassMap', { 'src/a.tsx': { color: 3 } }],
      ['importFileRoutes', { 'src/a.tsx': [0, 2] }],
      ['registerShorthands', { insetX: ['left', 'right'] }],
      ['registerTheme', { colors: { default: { primary: 'red' } } }],
    ])
      w[name](value)
    assert.equal(JSON.parse(w.exportFileMap())['src/a.tsx'], 4)
    assert.equal(JSON.parse(w.exportClassMap())['src/a.tsx'].color, 3)
    const packet = w.exportSheet()
    w.importSheet(JSON.parse(packet))
    assert.equal(w.exportSheet(), packet)
    outputs.push(h.read(h.open({ code: box('insetX={2} color="$primary"') })))
    return outputs
  })
  for (const kind of ['map', 'enum']) {
    await h.test(`${kind}-dynamic-receiver-enumeration`, () => {
      const fixture = h.fixture(kind),
        events = []
      const base =
        kind === 'map'
          ? {
              get a() {
                events.push('value')
                return 2
              },
            }
          : fixture.target
      if (kind === 'map') {
        Object.setPrototypeOf(base, {
          get inherited() {
            return assert.fail('Inherited values are not entries')
          },
        })
        Object.defineProperty(base, 'hidden', {
          get() {
            return assert.fail('Nonenumerable values are not read')
          },
        })
      }
      const target = new Proxy(base, {
        get(value, key, receiver) {
          events.push(`get:${String(key)}`)
          return Reflect.get(value, key, receiver)
        },
        ownKeys(value) {
          events.push('keys')
          return Reflect.ownKeys(value)
        },
        getOwnPropertyDescriptor(value, key) {
          events.push(`descriptor:${String(key)}`)
          return Reflect.getOwnPropertyDescriptor(value, key)
        },
      })
      if (kind === 'enum')
        Object.defineProperty(base, Symbol.iterator, {
          get() {
            return assert.fail('Enums must ignore Symbol.iterator')
          },
          configurable: true,
        })
      if (kind === 'enum') fixture.target = target
      const original = Object.entries
      h.patch(
        {
          get() {
            events.push('lookup')
            return function (value) {
              if (value === target || value === fixture.target) {
                assert.equal(this, Object)
                events.push('call')
                return original(target)
              }
              return Reflect.apply(original, this, [value])
            }
          },
        },
        () => fixture.invoke(kind === 'map' ? target : fixture.target),
      )
      const key = kind === 'map' ? 'a' : 'Selector'
      assert.equal(events[events.indexOf('call') - 1], 'lookup')
      assert.deepEqual(
        events.filter((event) => event !== 'lookup'),
        [
          ...(kind === 'map' ? ['get:Symbol(Symbol.iterator)'] : []),
          'call',
          'keys',
          `descriptor:${key}`,
          `get:${key}`,
          ...(kind === 'map' ? ['value', 'descriptor:hidden'] : []),
        ],
      )
      return events
    })
    await h.test(`${kind}-length-snapshot-mutation`, () => {
      const fixture = h.fixture(kind),
        events = [],
        pair = kind === 'map' ? ['a', 7] : fixture.pair
      let bound = 1
      const entries = indexed((value, key) => {
        events.push(String(key))
        if (key === 'length') {
          if (bound === 0) return 0
          return {
            valueOf() {
              events.push('coerce')
              bound = 0
              value[0] = pair
              return 1.9
            },
          }
        }
        assert.equal(key, '0', 'No eager iteration or out-of-snapshot access')
        assert.equal(bound, 0)
        return value[0]
      })
      h.entries(fixture.target, entries, () => fixture.invoke(fixture.target))
      assert.deepEqual(events, ['length', 'coerce', '0'])
      if (kind === 'map')
        assert.deepEqual(JSON.parse(w.exportFileMap()), { a: 7 })
      return events
    })
    for (const [label, length, count] of [
      ['zero', 0, 0],
      ['fraction', 1.9, 1],
      ['negative-wrap-one', -4294967295, 1],
      ['wrap-zero', 4294967296, 0],
      ['wrap-one', 4294967297, 1],
      ['string-one', '1', 1],
      ['negative-huge', -1, 4294967295],
    ])
      await h.test(`${kind}-length-${label}`, () => {
        const fixture = h.fixture(kind),
          events = [],
          stop = {}
        const entries = indexed((_value, key) => {
          events.push(String(key))
          if (key === 'length') return length
          assert.equal(key, '0')
          if (count > 1) throw stop
          return kind === 'map' ? ['a', 5] : fixture.pair
        })
        const result = h.take(() =>
          h.entries(fixture.target, entries, () =>
            fixture.invoke(fixture.target),
          ),
        )
        if (kind === 'enum' && count !== 1) {
          assert.equal(result.caught, true)
          assert.equal(result.error.constructor, TypeError)
          assert.equal(result.error.message, slot.sheet)
        } else if (count > 1) {
          assert.equal(result.caught, true)
          assert.equal(result.error, stop)
        } else assert.equal(result.caught, false)
        assert.deepEqual(events, [
          'length',
          ...(kind === 'enum' && count !== 1 ? ['length'] : []),
          ...(count > 0 && (kind === 'map' || count === 1) ? ['0'] : []),
        ])
        return events
      })
    for (const [label, make, valid] of [
      ['empty-string', () => '', true],
      ['string', () => 'xy', false],
      ['number', () => 4, true],
      ['boolean', () => true, true],
      ['bigint', () => 2n, true],
      ['symbol', () => Symbol('E'), true],
      ['promise', () => Promise.resolve([]), true],
      ['null', () => null, false],
      ['undefined', () => undefined, false],
    ])
      await h.test(`${kind}-result-${label}`, () => {
        const fixture = h.fixture(kind)
        const result = h.take(() =>
          h.entries(fixture.target, make(), () =>
            fixture.invoke(fixture.target),
          ),
        )
        const succeeds = kind === 'map' && valid
        assert.equal(result.caught, !succeeds)
        if (result.caught) assert.equal(result.error.constructor, TypeError)
        return { caught: result.caught }
      })
  }
  await h.test('iterator-replaces-dynamic-entries', () => {
    const events = [],
      target = {}
    Object.defineProperty(target, Symbol.iterator, {
      get() {
        events.push('iterator')
        Object.defineProperty(Object, 'entries', {
          configurable: true,
          value(value) {
            assert.equal(this, Object)
            assert.equal(value, target)
            events.push('replacement')
            return [['a', 9]]
          },
        })
        return undefined
      },
    })
    h.patch(
      {
        value() {
          assert.fail('Stale entries function used')
        },
      },
      () => w.importFileMap(target),
    )
    assert.deepEqual(events, ['iterator', 'replacement'])
    assert.deepEqual(JSON.parse(w.exportFileMap()), { a: 9 })
    return events
  })
  await h.test('enum-zero-multiple-no-index', () => {
    const fixture = h.fixture('enum'),
      events = []
    for (const length of [0, 2]) {
      h.schema(
        () =>
          h.entries(
            fixture.target,
            indexed((_value, key) => {
              events.push(String(key))
              assert.equal(key, 'length')
              return length
            }),
            () => fixture.invoke(),
          ),
        slot.sheet,
      )
    }
    assert.deepEqual(events, ['length', 'length', 'length', 'length'])
    return events
  })
}
