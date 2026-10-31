import assert from 'node:assert/strict'

import { pkg, source } from './corpus/extraction.mjs'

export const slot = {
  file: 'importFileMap(): argument sheet_object must be a file-index mapping',
  classes:
    'importClassMap(): argument sheet_object must be a nested class-index mapping',
  sheet: 'importSheet(): argument sheet_object must be a sheet state',
  rows: 'reserveClassSlots(): argument rows must be an iterable of 3-string rows',
  aliases:
    'reserveClassSlots(): argument import_aliases must be an alias mapping',
  prefix: 'setPrefix(): argument prefix must be a string, null or undefined',
  filename: 'codeExtract(): argument filename must be a string',
  extraction: 'codeExtract(): argument import_aliases must be an alias mapping',
}

export const wrappedPhases = [
  'iterator-get',
  'iterator-call',
  'next-get',
  'next-call',
]
export const iteratorPhases = [
  'iterator-get',
  'iterator-call',
  'initial-next',
  'next-get',
  'next-call',
  'done',
  'value',
]

export function indexed(get) {
  return new Proxy({}, { get })
}

export function queueProbe(target, { phase, mark, enqueue }) {
  if (!['iterator', 'nonfunction-method', 'noncallable-next'].includes(phase))
    return undefined
  const event = phase === 'iterator' ? 'iterator' : 'probe'
  const probe = () => {
    mark(event)
    enqueue()
  }
  Object.defineProperty(target, Symbol.iterator, {
    get() {
      if (phase === 'noncallable-next')
        return () => ({
          get next() {
            probe()
            return 3
          },
        })
      probe()
      return phase === 'iterator' ? undefined : 3
    },
  })
  return event
}

export function reserve(w, rows, aliases) {
  return w.reserveClassSlots(rows, pkg, false, false, false, aliases)
}

export function iteratorFailure(phase, fail) {
  let reads = 0
  const iterator = {
    get next() {
      reads++
      if (phase === 'initial-next' || (phase === 'next-get' && reads === 2))
        return fail()
      return function () {
        if (phase === 'next-call') return fail()
        return {
          get done() {
            return phase === 'done' ? fail() : false
          },
          get value() {
            return fail()
          },
        }
      }
    },
  }
  return {
    get [Symbol.iterator]() {
      if (phase === 'iterator-get') return fail()
      return function () {
        return phase === 'iterator-call' ? fail() : iterator
      }
    },
  }
}

export async function runAliasRaw(h) {
  const { w } = h
  for (const phase of [
    'lookup',
    'call',
    'enumeration',
    'length',
    'coercion',
    'index',
    'pair-0',
    'pair-1',
    'initial-next',
    'done',
    'value',
    'diagnostic',
  ]) {
    for (const sentinelName of ['undefined', 'null', 'hostile'])
      await h.test(`aliases-raw-${phase}-${sentinelName}`, () => {
        const events = []
        const sentinel =
          sentinelName === 'undefined'
            ? undefined
            : sentinelName === 'null'
              ? null
              : {
                  [Symbol.toPrimitive]() {
                    assert.fail('Raw alias String')
                  },
                }
        const fail = () => {
          events.push(phase)
          throw sentinel
        }
        const aliases = ['initial-next', 'done', 'value'].includes(phase)
          ? iteratorFailure(phase, fail)
          : {}
        w.setModuleResolver(() => {
          events.push('resolver')
          assert.fail('Raw alias reached resolver')
        })
        if (phase === 'enumeration')
          Object.defineProperty(aliases, 'bad', { enumerable: true, get: fail })
        if (phase === 'diagnostic')
          aliases.bad = {
            get [Symbol.toStringTag]() {
              return fail()
            },
          }
        const entries = indexed((_value, key) => {
          if (key === 'length') {
            if (phase === 'length') return fail()
            return phase === 'coercion' ? { valueOf: fail } : 1
          }
          assert.equal(key, '0')
          if (phase === 'index') return fail()
          return indexed((_pair, part) => {
            if (part === phase.slice(5)) return fail()
            assert.equal(part, '0')
            return 'unused'
          })
        })
        const original = Object.entries
        const descriptor =
          phase === 'lookup'
            ? { get: fail }
            : {
                value(value) {
                  if (value !== aliases)
                    return Reflect.apply(original, this, [value])
                  if (phase === 'call') return fail()
                  return ['enumeration', 'diagnostic'].includes(phase)
                    ? original(value)
                    : entries
                },
              }
        h.raw(
          () =>
            h.patch(descriptor, () =>
              w.hasDevupUI(
                'src/raw-alias.tsx',
                "import {Box} from './barrel'; export const v=<Box/>",
                pkg,
                aliases,
              ),
            ),
          sentinel,
        )
        assert.deepEqual(events, [phase])
        return events
      })
  }
}

export async function runAliasHostile(h) {
  const { w } = h
  for (const phase of wrappedPhases)
    for (const conversion of ['toString', 'primitive'])
      await h.test(`aliases-hostile-${phase}-${conversion}`, () => {
        const events = [],
          fault = {}
        const convert = () => {
          events.push('String')
          throw fault
        }
        const error =
          conversion === 'toString'
            ? { toString: convert }
            : { [Symbol.toPrimitive]: convert }
        const aliases = iteratorFailure(phase, () => {
          events.push(phase)
          throw error
        })
        w.setModuleResolver(() => {
          events.push('resolver')
          assert.fail('Conversion fault reached resolver')
        })
        h.raw(
          () =>
            w.hasDevupUI(
              'src/hostile.tsx',
              "import {Box} from './barrel'; export const v=<Box/>",
              pkg,
              aliases,
            ),
          fault,
        )
        assert.deepEqual(events, [phase, 'String'])
        w.setModuleResolver(null)
        assert.equal(w.hasDevupUI('src/clean.tsx', source, pkg, {}), true)
        const later = {}
        h.raw(
          () =>
            w.importFileMap({
              get bad() {
                throw later
              },
            }),
          later,
        )
        return events
      })
}
