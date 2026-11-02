import assert from 'node:assert/strict'
import { AsyncLocalStorage } from 'node:async_hooks'

import { pkg, source } from './corpus/extraction.mjs'
import {
  indexed,
  iteratorFailure,
  queueProbe,
  reserve,
  slot,
  wrappedPhases,
} from './entries-fixtures.mjs'

export async function runAliasConversions(h, barrel, resolved) {
  const { w } = h
  for (const phase of wrappedPhases)
    for (const mutation of ['keep', 'install', 'clear'])
      await h.test(`aliases-conversion-${phase}-${mutation}`, () => {
        const events = [],
          inner = {}
        const resolver = () => {
          events.push('resolver')
          return resolved
        }
        w.setModuleResolver(mutation === 'install' ? null : resolver)
        const error = {
          toString() {
            events.push('String')
            h.schema(() => w.importClassMap({ a: { color: -1 } }), slot.classes)
            h.raw(
              () =>
                w.hasDevupUI('src/inner.tsx', source, pkg, {
                  get bad() {
                    throw inner
                  },
                }),
              inner,
            )
            events.push('nested')
            if (mutation !== 'keep')
              w.setModuleResolver(mutation === 'install' ? resolver : null)
            return 'forgiven'
          },
        }
        const aliases = () =>
          iteratorFailure(phase, () => {
            events.push(phase)
            throw error
          })
        assert.equal(
          w.hasDevupUI('src/converted.tsx', barrel, pkg, aliases()),
          mutation !== 'clear',
        )
        assert.deepEqual(events, [
          phase,
          'String',
          'nested',
          ...(mutation === 'clear' ? [] : ['resolver']),
        ])
        const before = events.length
        assert.equal(
          w.hasDevupUI('src/empty.tsx', barrel, pkg, {}),
          mutation !== 'clear',
        )
        assert.deepEqual(
          events.slice(before),
          mutation === 'clear' ? [] : ['resolver'],
        )
        w.setModuleResolver(null)
        for (const action of [
          () => h.open({ aliases: aliases() }),
          () =>
            h.open({ mode: 'codeExtractWithoutSourceMap', aliases: aliases() }),
          () => reserve(w, [], aliases()),
        ]) {
          const start = events.length,
            result = h.take(action)
          assert.equal(result.caught, true)
          assert.equal(result.error.constructor, Error)
          assert.equal(result.error.message, 'forgiven')
          assert.deepEqual(events.slice(start), [phase, 'String', 'nested'])
          w.setModuleResolver(null)
        }
        return events
      })
}

export async function runResolverOrder(h) {
  for (const [id, imports, use, expected] of [
    [
      'resolver-order-declarations',
      "import {shade} from './two'; import {color} from './one';",
      '<Box bg={shade} color={color}/>',
      ['./two', './one', './one', './two'],
    ],
    [
      'resolver-order-first-use',
      "import {color} from './one'; import {shade} from './two';",
      '<Box bg={shade} color={color}/>',
      ['./one', './two', './one', './two'],
    ],
    [
      'resolver-order-local-aliases',
      "import {color as shade} from './one'; import {shade as color} from './two';",
      '<Box color={shade} bg={color}/>',
      ['./one', './two', './two', './one'],
    ],
  ]) {
    await h.test(id, () => {
      const events = []
      h.w.setModuleResolver(function (specifier, importer) {
        assert.equal(this, null)
        events.push([specifier, importer])
        return {
          path: `src/${specifier.slice(2)}.ts`,
          code: 'export const color="purple"; export const shade="blue";',
        }
      })
      const output = h.read(
        h.open({
          code: `import {Box} from '${pkg}'; ${imports} export const v=${use}`,
          file: 'src/order.tsx',
        }),
      )
      assert.deepEqual(
        events,
        expected.map((specifier) => [specifier, 'src/order.tsx']),
      )
      return { events, output }
    })
  }
}

export async function runQueues(h, phases = ['entries', 'iterator', 'enum']) {
  const { w } = h
  for (const phase of phases) {
    for (const queue of ['Promise', 'queueMicrotask', 'nextTick']) {
      await h.test(`queue-context-${phase}-${queue}`, async () => {
        const storage = new AsyncLocalStorage(),
          events = [],
          fixture = h.fixture(phase === 'enum' ? 'enum' : 'map')
        const target = fixture.target,
          original = Object.entries
        let length = 1,
          finish
        const drained = new Promise((resolve) => {
          finish = resolve
        })
        const mark = (event) => events.push([event, storage.getStore()])
        const enqueue = () => {
          const callback = () => {
            mark('queue')
            length = 0
            finish()
          }
          switch (queue) {
            case 'Promise':
              void Promise.resolve().then(callback)
              break
            case 'queueMicrotask':
              queueMicrotask(callback)
              break
            case 'nextTick':
              process.nextTick(callback)
              break
            default:
              assert.fail(queue)
          }
        }
        const entries = indexed((_value, key) => {
          if (key === 'length') {
            mark('length')
            return length
          }
          assert.equal(key, '0')
          mark('index')
          return phase === 'enum' ? fixture.pair : ['a', 8]
        })
        const probe = queueProbe(target, { phase, mark, enqueue })
        storage.run('outer-context', () => {
          h.patch(
            {
              value(value) {
                if (value !== target)
                  return Reflect.apply(original, this, [value])
                mark('entries')
                if (probe === undefined) enqueue()
                storage.run('nested-context', () => {
                  h.schema(() => w.setPrefix(false), slot.prefix)
                  assert.equal(storage.getStore(), 'nested-context')
                })
                return entries
              },
            },
            () => fixture.invoke(target),
          )
          mark('after')
          assert.deepEqual(
            events.map(([event]) => event),
            [
              ...(probe === undefined ? [] : [probe]),
              'entries',
              'length',
              'index',
              'after',
            ],
          )
        })
        await drained
        assert.deepEqual(
          events,
          [
            ...(probe === undefined ? [] : [probe]),
            'entries',
            'length',
            'index',
            'after',
            'queue',
          ].map((event) => [event, 'outer-context']),
        )
        assert.equal(storage.getStore(), undefined)
        if (phase !== 'enum')
          assert.deepEqual(JSON.parse(w.exportFileMap()), { a: 8 })
        storage.disable()
        return events
      })
    }
  }
}
