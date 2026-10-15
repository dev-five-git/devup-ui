import { expect, it } from 'bun:test'
import type { UserConfig } from 'vite'

import { createAggregateCssPreparation } from '../aggregate-css'
import { DevupUI } from '../plugin'
import { fixture, module } from './aggregate-fixture'

type TestPlugin = {
  buildStart(options: { input: string }): void | Promise<void>
  config(this: void, config: UserConfig): Omit<UserConfig, 'plugins'>
  closeBundle(): void
}

it('shares one complete preparation when concurrent sheets use multiple cyclic entries', async () => {
  // Given: two roots, a static cycle, dynamic and extensionless virtual edges.
  const f = fixture({ first: 'entry', second: 'other' })
  f.modules.set(
    'entry',
    module(
      "import 'cycle'; export const lazy=()=>import('dynamic')",
      ['cycle'],
      ['dynamic'],
    ),
  )
  f.modules.set(
    'cycle',
    module("import 'entry'; import 'sheet.css'", ['entry', 'sheet.css']),
  )
  f.modules.set('other', module("import 'virtual'", ['virtual']))
  f.modules.set('dynamic', module())
  f.modules.set('virtual', module("import 'injected'", ['injected']))
  f.modules.set('injected', module())
  // When: every first snapshot requests the same environment barrier.
  const first = f.preparation.prepare(f.environment, f.context, 'aggregate.css')
  const second = f.preparation.prepare(f.environment, f.context, 'numbered.css')
  await Promise.all([first, second])
  // Then: exactly the reachable non-CSS closure is ready, without a cycle wait.
  expect(first).toBe(second)
  expect([...f.loaded].sort()).toEqual([
    'cycle',
    'dynamic',
    'entry',
    'injected',
    'other',
    'virtual',
  ])
})

it('isolates and resets barriers when builds reuse environment objects', async () => {
  const f = fixture(['entry'])
  const secondEnvironment = {}
  f.modules.set('other', module())
  f.preparation.start(secondEnvironment, ['other'], { cssCodeSplit: false })
  const first = f.preparation.prepare(f.environment, f.context, 'first.css')
  const second = f.preparation.prepare(
    secondEnvironment,
    f.context,
    'second.css',
  )
  await Promise.all([first, second])
  expect(first).not.toBe(second)
  expect([...f.loaded].sort()).toEqual(['entry', 'other'])
  f.preparation.start(f.environment, 'other', { cssCodeSplit: false })
  const rebuilt = f.preparation.prepare(f.environment, f.context, 'rebuilt.css')
  expect(rebuilt).not.toBe(first)
  await rebuilt
})

it('leaves ordinary application builds untouched when aggregation is disabled', () => {
  const f = fixture()
  f.preparation.start(f.environment, 'entry', { cssCodeSplit: true })
  expect(
    f.preparation.prepare(f.environment, f.context, 'sheet.css'),
  ).toBeUndefined()
  expect(f.loaded.size).toBe(0)
})

it('uses public external resolution when cached module information lacks isExternal', async () => {
  const f = fixture()
  f.modules.set(
    'entry',
    module("import 'external'; import 'rollup-external'; import 'virtual';", [
      'external',
      'rollup-external',
      'virtual',
    ]),
  )
  f.cached.set('external', { ...module(), code: null })
  f.cached.set('rollup-external', { ...module(), isExternal: true })
  f.cached.set('virtual', { ...module(), code: null })
  f.modules.set('virtual', module())
  f.context.resolve = async (id) =>
    id === 'virtual' ? null : { id, external: id === 'external' }
  await f.preparation.prepare(f.environment, f.context, 'aggregate.css')
  expect([...f.loaded]).toEqual(['entry', 'virtual'])
})

it('honors public canonical IDs when resolution redirects a dependency', async () => {
  const f = fixture()
  f.modules.set('entry', module("import 'alias'", ['alias']))
  f.modules.set('canonical', module())
  f.context.resolve = async (id) => ({
    id: id === 'alias' ? 'canonical' : id,
    external: false,
  })
  await f.preparation.prepare(f.environment, f.context, 'aggregate.css')
  expect([...f.loaded]).toEqual(['entry', 'canonical'])
})

it('accepts literal and empty-template dynamic imports after transformation', async () => {
  const f = fixture()
  f.modules.set(
    'entry',
    module(
      "export const a=()=>import('literal'); export const b=()=>import(`template`)",
      [],
      ['literal', 'template'],
    ),
  )
  f.modules.set('literal', module())
  f.modules.set('template', module())
  await f.preparation.prepare(f.environment, f.context, 'aggregate.css')
  expect([...f.loaded]).toEqual(['entry', 'literal', 'template'])
})

it('keeps aggregation out of dev and disabled-extraction build hooks', () => {
  const [plugin] = DevupUI({ extractCss: false }) as unknown as [TestPlugin]
  Reflect.apply(plugin.buildStart, {}, [{ input: 'entry' }])
  expect(
    plugin.config.call(undefined, { build: { lib: { entry: 'entry' } } }).build,
  ).toBeUndefined()
  plugin.closeBundle()
})

it('records fallback build inputs without preloading in buildStart', () => {
  const [plugin] = DevupUI() as unknown as [TestPlugin]
  const f = fixture()
  Reflect.apply(plugin.buildStart, f.context, [{ input: 'entry' }])
  expect(f.loaded.size).toBe(0)
  expect(
    plugin.config.call(undefined, { build: { cssCodeSplit: false } }).build,
  ).toBeUndefined()
  plugin.closeBundle()
})

it('fails located when a source transform awaits the generated CSS barrier it blocks', async () => {
  const expirations: (() => void)[] = []
  const f = fixture()
  const preparation = createAggregateCssPreparation((expire) => {
    expirations.push(expire)
    return () => {}
  })
  preparation.start(f.environment, 'entry', { cssCodeSplit: false })
  const entered = Promise.withResolvers<void>()
  f.context.load = async () => {
    entered.resolve()
    await preparation.prepare(f.environment, f.context, 'aggregate.css')
    return module()
  }
  const pending = preparation.prepare(f.environment, f.context, 'aggregate.css')
  await entered.promise
  for (const expire of expirations) expire()
  await expect(pending).rejects.toThrow(
    'entry:1:1: [devup-ui] aggregate CSS asset aggregate.css',
  )
  await expect(pending).rejects.toThrow('preparation cycle')
})

it('prepares actual plugin-defined entry modules but never cached orphan modules', async () => {
  const f = fixture()
  f.cached.set('plugin-entry', { ...module(), isEntry: true })
  f.cached.set('orphan', module())
  f.modules.set('plugin-entry', module())
  f.modules.set('orphan', module())
  await f.preparation.prepare(f.environment, f.context, 'aggregate.css')
  expect([...f.loaded]).toEqual(['entry', 'plugin-entry'])
  expect(() =>
    f.preparation.observe(f.environment, 'plugin-entry'),
  ).not.toThrow()
  expect(() => f.preparation.observe(f.environment, 'late')).toThrow(
    'late:1:1: [devup-ui] aggregate CSS asset aggregate.css',
  )
})
