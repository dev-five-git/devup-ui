import * as fs from 'node:fs/promises'

import { expect, it, spyOn } from 'bun:test'
import { type build, createBuilder } from 'vite'

import { DevupUI } from '../plugin'
import { productionFixture } from './production-fixture'

type NativeWatcher = Extract<
  Awaited<ReturnType<typeof build>>,
  { readonly on: unknown }
>

it('drains shared config refresh when an idle native sibling watcher closes', async () => {
  // Given native sibling watchers sharing one unchanged Devup tuple.
  const fixture = await productionFixture()
  const write = fs.writeFile
  const events: string[] = []
  const watchers: NativeWatcher[] = []
  const requested = Promise.withResolvers<void>()
  let idle: NativeWatcher | undefined
  let closing: Promise<void> | undefined
  let armed = false
  let observingEdit = false
  let idleRebuilt = false
  let restoreHook: (() => void) | undefined
  const observer = spyOn(fs, 'writeFile').mockImplementation(
    (path, data, options) => {
      const promise = write(path, data, options)
      if (
        armed &&
        idle &&
        typeof path === 'string' &&
        path.replaceAll('\\', '/') === `${fixture.root}/df/theme.d.ts`
      ) {
        armed = false
        events.push('write-submitted')
        void promise.then(() => events.push('write-settled'), requested.reject)
        events.push('close-requested')
        closing = idle.close()
        void closing.then(() => events.push('close-settled'), requested.reject)
        requested.resolve()
      }
      return promise
    },
  )
  try {
    const first = await fixture.file(
      'app/src/a.js',
      "import {css} from '@devup-ui/react'; export const cls=css({background:'red'});",
    )
    const second = await fixture.file(
      'app/src/b.js',
      "import {css} from '@devup-ui/react'; export const cls=css({background:'blue'});",
    )
    await fixture.file('app/devup.json', '{}')
    const tuple = DevupUI()
    const builder = await createBuilder({
      root: fixture.root,
      configFile: false,
      logLevel: 'silent',
      plugins: [tuple],
      environments: {
        client: {
          build: {
            watch: { include: [first] },
            lib: { entry: first, formats: ['es'] },
          },
        },
        sibling: {
          consumer: 'client',
          build: { watch: {}, lib: { entry: second, formats: ['es'] } },
        },
      },
      build: {
        write: false,
        watch: {},
        lib: { entry: first, formats: ['es'] },
      },
    })
    const main = builder.config.plugins.find(
      (plugin) => plugin.name === 'devup-ui',
    )
    const hook = main?.closeWatcher
    if (!main || typeof hook !== 'function')
      throw new TypeError('Expected shared native closeWatcher hook')
    const hookObserver = spyOn(main, 'closeWatcher').mockImplementation(
      function (this: ThisParameterType<typeof hook>, ...args) {
        const result = hook.apply(this, args)
        if (observingEdit) {
          events.push(
            result instanceof Promise ? 'hook-pending' : 'hook-complete',
          )
        }
        return result
      },
    )
    restoreHook = () => hookObserver.mockRestore()
    for (const name of ['client', 'sibling']) {
      const environment = builder.environments[name]
      if (!environment) throw new TypeError('Expected native sibling')
      const watcher = await builder.build(environment)
      if (Array.isArray(watcher) || !('on' in watcher))
        throw new TypeError('Expected native watcher')
      watchers.push(watcher)
      const ready = Promise.withResolvers<void>()
      watcher.on('event', (event) => {
        if (event.code === 'END') ready.resolve()
        if (event.code === 'ERROR') {
          ready.reject(event.error)
          requested.reject(event.error)
        }
        if (observingEdit && name === 'client' && event.code === 'BUNDLE_START')
          idleRebuilt = true
      })
      await ready.promise
      if (name === 'client') idle = watcher
    }
    // When B refreshes real theme bytes, request A's native close at I/O submission.
    observingEdit = true
    armed = true
    await write(
      `${fixture.root}/devup.json`,
      '{"theme":{"colors":{"default":{"primary":"green"}}}}',
    )
    await requested.promise
    await closing
    // Then native close awaits a pending shared hook and the submitted real write.
    expect(idleRebuilt).toBe(false)
    expect(events.indexOf('hook-pending')).toBeGreaterThan(
      events.indexOf('close-requested'),
    )
    expect(events.indexOf('write-settled')).toBeGreaterThan(
      events.indexOf('write-submitted'),
    )
    expect(events.indexOf('close-settled')).toBeGreaterThan(
      events.indexOf('hook-pending'),
    )
    expect(events.indexOf('close-settled')).toBeGreaterThan(
      events.indexOf('write-settled'),
    )
    console.info(
      JSON.stringify({ event: 'native-close-watcher-drain', events }),
    )
  } finally {
    armed = false
    observingEdit = false
    try {
      await Promise.all(
        watchers.map((watcher) =>
          watcher === idle && closing ? closing : watcher.close(),
        ),
      )
    } finally {
      restoreHook?.()
      observer.mockRestore()
      await fixture.close()
    }
  }
}, 30000)
