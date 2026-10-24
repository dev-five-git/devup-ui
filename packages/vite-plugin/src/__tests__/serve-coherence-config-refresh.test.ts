import { rmSync } from 'node:fs'
import * as fs from 'node:fs/promises'
import { join } from 'node:path'

import { ConfigLoadError } from '@devup-ui/plugin-utils'
import { getCss } from '@devup-ui/wasm'
import { expect, it, mock, spyOn } from 'bun:test'

import {
  environment,
  graph,
  lifecycle,
  watcher,
} from './plugin-lifecycle-fixture'

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
      const setup = state.plugin.watchChange(
        join(state.project.root, 'devup.json'),
      )
      if (action === 'recreate')
        await expect(setup).rejects.toBeInstanceOf(ConfigLoadError)
      else await setup
      const watching = watcher()
      state.plugin.configureServer({ config: state.config, watcher: watching })
      const path = action === 'root-delete' ? 'devup.json' : 'base.json'
      const file = join(state.project.root, path)
      expect(watching.batches.flat()).toContain(file)
      const source =
        "import {Box} from '@devup-ui/react'; export const A=<Box w='$refreshSize'/>"
      const extract = () =>
        state.plugin.transform.call(
          { addWatchFile() {} },
          source,
          join(state.project.root, 'src/A.tsx'),
        )
      if (action === 'recreate')
        await expect(extract()).rejects.toThrow(`${file}:1:1:`)
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
      const refresh = async () => {
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
      }
      if (action === 'delete') {
        await expect(refresh()).rejects.toBeInstanceOf(ConfigLoadError)
        await expect(extract()).rejects.toThrow(`${file}:1:1:`)
        await state.plugin.watchChange(
          join(state.project.root, 'src/unrelated.tsx'),
        )
        await expect(extract()).rejects.toThrow(`${file}:1:1:`)
        expect(watching.batches.flat()).toContain(file)
        state.project.write(path, theme('37px'))
        await refresh()
        expect(await extract()).toBeDefined()
        expect(getCss(null, false)).toContain('--refreshSize:37px')
        return
      }
      await refresh()
      if (action === 'recreate') expect(await extract()).toBeDefined()
      // Then
      const css = getCss(null, false)
      switch (action) {
        case 'edit':
        case 'recreate':
          expect(css).toContain('--refreshSize:29px')
          break
        case 'root-delete':
          expect(css).not.toContain('--refreshSize:')
          break
        default:
          throw new TypeError(`Unexpected action ${action satisfies never}`)
      }
    })
  }
}

it('retains malformed extended JSON failure until the actual input is repaired', async () => {
  // Given
  using state = await lifecycle()
  state.project.write('devup.json', '{"extends":["./base.json"]}')
  state.project.write(
    'base.json',
    '{"theme":{"colors":{"default":{"primary":"red"}}}}',
  )
  await state.plugin.watchChange(join(state.project.root, 'devup.json'))
  const watching = watcher()
  state.plugin.configureServer({ config: state.config, watcher: watching })
  const file = join(state.project.root, 'base.json')
  const extract = () =>
    state.plugin.transform.call(
      { addWatchFile() {} },
      "import {Box} from '@devup-ui/react'; export const A=<Box color='$primary'/>",
      join(state.project.root, 'src/A.tsx'),
    )
  await extract()
  // When
  state.project.write('base.json', '{')
  await expect(state.plugin.watchChange(file)).rejects.toBeInstanceOf(
    ConfigLoadError,
  )
  // Then
  await expect(extract()).rejects.toThrow(`${file}:1:1:`)
  const load = Reflect.get(state.plugin, 'load')
  if (typeof load !== 'function') throw new TypeError('Expected load hook')
  expect(() =>
    Reflect.apply(load, {}, [join(state.project.root, 'df/css/devup-ui.css')]),
  ).toThrow(`${file}:1:1:`)
  expect(watching.batches.flat()).toContain(file)
  state.project.write(
    'base.json',
    '{"theme":{"colors":{"default":{"primary":"blue"}}}}',
  )
  await state.plugin.watchChange(file)
  expect(await extract()).toBeDefined()
  expect(getCss(null, false)).toContain('--primary:blue')
})

it('waits for queued refresh before extracting unchanged consumer bytes', async () => {
  // Given
  using state = await lifecycle()
  const source =
    "import {Box} from '@devup-ui/react'; export const A=<Box w='$size'/>"
  state.project.write(
    'devup.json',
    '{"theme":{"length":{"default":{"size":"11px"}}}}',
  )
  await state.plugin.watchChange(join(state.project.root, 'devup.json'))
  const extract = () =>
    state.plugin.transform.call(
      { addWatchFile() {} },
      source,
      join(state.project.root, 'src/A.tsx'),
    )
  const before = await extract()
  const entered = Promise.withResolvers<void>()
  const release = Promise.withResolvers<void>()
  const read = fs.readFile
  using _reading = spyOn(fs, 'readFile').mockImplementation(async (...args) => {
    if (args[0] === join(state.project.root, 'devup.json')) {
      entered.resolve()
      await release.promise
    }
    return read(...args)
  })
  state.project.write(
    'devup.json',
    '{"theme":{"length":{"default":{"size":["11px",null,"29px"]}}}}',
  )
  const refresh = state.plugin.watchChange(
    join(state.project.root, 'devup.json'),
  )
  await entered.promise
  // When
  const pending = extract()
  release.resolve()
  await refresh
  // Then
  expect(await pending).not.toEqual(before)
  expect(getCss(null, false)).toContain('29px')
})
