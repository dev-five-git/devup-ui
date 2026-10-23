import { join } from 'node:path'

import type { ReservationRow } from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import { expect, it, mock, spyOn } from 'bun:test'

import { STYLE } from '../../../plugin-utils/src/__tests__/reservation-fixture'
import {
  environment,
  graph,
  lifecycle,
  watcher,
} from './plugin-lifecycle-fixture'

it('reserves actual server environment union and detaches each watcher lifetime on replacement and close', async () => {
  // Given
  using reserve = spyOn(wasm, 'reserveClassSlots').mockImplementation(() => {})
  using state = await lifecycle()
  const { project, plugin, config } = state
  project.write('physical/A.tsx', STYLE)
  project.write('physical/B.tsx', STYLE)
  const first = watcher()
  const second = watcher()
  const environments = Object.fromEntries(
    ['A', 'B'].map((name) => [
      name,
      {
        name,
        config: { input: `physical/${name}.tsx` },
      },
    ]),
  )
  plugin.configureServer({ config, environments, watcher: first })
  const addWatchFile = mock()
  // When
  await plugin.buildStart.call({ addWatchFile })
  plugin.configureServer({ config, environments, watcher: second })
  await plugin.buildStart.call({ addWatchFile })
  // Then
  const files = reserve.mock.calls.flatMap((call) =>
    call[0].map((row: ReservationRow) => row[0]),
  )
  for (const name of ['A', 'B']) {
    const file = join(project.root, `physical/${name}.tsx`).replaceAll(
      '\\',
      '/',
    )
    expect(files).toContain(file)
    expect(addWatchFile).toHaveBeenCalledWith(file)
    expect(second.batches.flat()).toContain(file)
  }
  expect(first.listenerCount('all')).toBe(0)
  expect(second.listenerCount('all')).toBe(1)
  plugin.closeBundle()
  expect(second.listenerCount('all')).toBe(0)
})

it('reserves actual buildApp environments before buildStart watches the discovered suppliers', async () => {
  // Given
  using reserve = spyOn(wasm, 'reserveClassSlots').mockImplementation(() => {})
  using state = await lifecycle()
  state.project.write('physical/A.tsx', STYLE)
  state.project.write('physical/B.tsx', STYLE)
  await state.plugin.buildApp({
    config: state.config,
    environments: {
      client: { name: 'client', config: { input: 'physical/A.tsx' } },
      ssr: { name: 'ssr', config: { input: 'physical/B.tsx' } },
    },
  })
  const addWatchFile = mock()
  // When
  await state.plugin.buildStart.call({ addWatchFile })
  // Then
  for (const name of ['A', 'B'])
    expect(addWatchFile).toHaveBeenCalledWith(
      join(state.project.root, `physical/${name}.tsx`).replaceAll('\\', '/'),
    )
  expect(reserve).toHaveBeenCalledTimes(1)
})

it('does not prepare or watch dependencies when extraction is disabled', async () => {
  // Given
  using reserve = spyOn(wasm, 'reserveClassSlots').mockImplementation(() => {})
  using state = await lifecycle({ extractCss: false })
  const addWatchFile = mock()
  state.plugin.configureServer({ config: state.config, watcher: watcher() })
  // When
  await state.plugin.buildStart.call({ addWatchFile })
  // Then
  expect(reserve).not.toHaveBeenCalled()
  expect(addWatchFile).not.toHaveBeenCalled()
})

it.each(['watchChange', 'hotUpdate', 'handleHotUpdate'] as const)(
  'watches new Tailwind inputs once when config refresh uses %s',
  async (hook) => {
    // Given
    using state = await lifecycle()
    const watching = watcher()
    state.plugin.configureServer({ config: state.config, watcher: watching })
    state.project.write('new-tailwind.css', '@theme { --color-brand: red; }')
    const file = join(state.project.root, 'new-tailwind.css')
    state.project.write(
      'devup.json',
      JSON.stringify({ tailwind: { css: file } }),
    )
    const event = {
      file: join(state.project.root, 'devup.json'),
      modules: [],
      timestamp: 1,
    }
    // When
    switch (hook) {
      case 'watchChange':
        await state.plugin.watchChange(event.file)
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
    expect(watching.batches.flat().filter((path) => path === file)).toEqual([
      file,
    ])
    expect(
      watching.batches.flat().filter((path) => path === event.file),
    ).toEqual([event.file])
  },
)
