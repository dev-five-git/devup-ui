import { readFileSync, rmSync } from 'node:fs'
import { join } from 'node:path'

import { loadCompiler } from '@devup-ui/plugin-utils'
import { expect, it, mock } from 'bun:test'

import {
  environment,
  graph,
  lifecycle,
  watcher,
} from './plugin-lifecycle-fixture'
import { cacheFixture, cssLoader, writeGate } from './source-cache-fixture'

const { getCss } = loadCompiler('wasm')

for (const hook of ['watchChange', 'hotUpdate', 'handleHotUpdate'] as const) {
  it(`publishes refreshed single CSS and invalidates its cached variants through ${hook}`, async () => {
    // Given: a real 17px disk publication and query-qualified cached CSS nodes.
    using state = await cacheFixture()
    const theme = (size: string) =>
      JSON.stringify({ theme: { length: { default: { size } } } })
    state.project.write('devup.json', '{"extends":["./theme.json"]}')
    state.project.write('theme.json', theme('17px'))
    const file = join(state.project.root, 'theme.json')
    await state.plugin.watchChange(join(state.project.root, 'devup.json'))
    await state.transform()
    expect(readFileSync(state.cssFile, 'utf8')).toContain('--size:17px')
    const css = state.cssFile.replaceAll('\\', '/')
    const env = environment('client', [
      `${css}?devup-revision=1`,
      `${css}?direct`,
      '/unrelated.js',
    ])
    const cached = [...env.moduleGraph.idToModuleMap.values()].slice(0, 2)
    expect(cssLoader(state.plugin)(cached[0].id)).toContain('--size:17px')
    using writes = writeGate(state.cssFile)
    writes.release()
    state.project.write('theme.json', theme('29px'))
    const event = { file, modules: [], timestamp: 7 }
    // When
    switch (hook) {
      case 'watchChange':
        await state.plugin.watchChange(file)
        break
      case 'hotUpdate':
        await state.plugin.hotUpdate.call({ environment: env }, event)
        break
      case 'handleHotUpdate':
        await state.plugin.handleHotUpdate({
          ...event,
          server: { moduleGraph: env.moduleGraph, ws: env.hot },
        })
        break
      default:
        throw new TypeError(`Unexpected hook ${hook satisfies never}`)
    }
    // Then: current disk bytes, targeted invalidation, no redundant publication.
    expect(readFileSync(state.cssFile, 'utf8')).toBe(getCss(null, false))
    expect(readFileSync(state.cssFile, 'utf8')).toContain('--size:29px')
    if (hook !== 'watchChange') {
      expect(
        env.moduleGraph.invalidateModule.mock.calls.map(([node]) => node),
      ).toEqual(cached)
    }
    const count = () =>
      writes.spy.mock.calls.filter(([path]) => String(path) === state.cssFile)
        .length
    const published = count()
    await state.plugin.watchChange(file)
    expect(count()).toBe(published)
  })
}

for (const hook of ['watchChange', 'hotUpdate', 'handleHotUpdate'] as const) {
  for (const action of ['edit', 'delete', 'recreate', 'root-delete'] as const) {
    it(`refreshes actual config inputs when ${hook} receives ${action}`, async () => {
      // Given
      using state = await lifecycle()
      const theme = (size: string) =>
        JSON.stringify({
          theme: { length: { default: { refreshSize: size } } },
        })
      state.project.write('devup.json', '{"extends":["./base.json"]}')
      if (action !== 'recreate') state.project.write('base.json', theme('11px'))
      await state.plugin.watchChange(join(state.project.root, 'devup.json'))
      const watching = watcher()
      state.plugin.configureServer({ config: state.config, watcher: watching })
      const path = action === 'root-delete' ? 'devup.json' : 'base.json'
      const file = join(state.project.root, path)
      expect(watching.batches.flat()).toContain(file)
      switch (action) {
        case 'edit':
        case 'recreate':
          state.project.write(path, theme('29px'))
          break
        case 'delete':
        case 'root-delete':
          rmSync(file)
          break
        default:
          throw new TypeError(`Unexpected action ${action satisfies never}`)
      }
      const event = { file, modules: [], timestamp: 1 }
      // When
      switch (hook) {
        case 'watchChange':
          await state.plugin.watchChange(file)
          break
        case 'hotUpdate':
          await state.plugin.hotUpdate.call(
            { environment: environment('client') },
            event,
          )
          break
        case 'handleHotUpdate':
          await state.plugin.handleHotUpdate({
            ...event,
            server: { moduleGraph: graph(), ws: { send: mock() } },
          })
          break
        default:
          throw new TypeError(`Unexpected hook ${hook satisfies never}`)
      }
      // Then
      const css = getCss(null, false)
      switch (action) {
        case 'edit':
        case 'recreate':
          expect(css).toContain('--refreshSize:29px')
          break
        case 'delete':
        case 'root-delete':
          expect(css).not.toContain('--refreshSize:')
          break
        default:
          throw new TypeError(`Unexpected action ${action satisfies never}`)
      }
    })
  }
}
