import { EventEmitter } from 'node:events'
import { join } from 'node:path'

import * as wasm from '@devup-ui/wasm'
import { expect, it, spyOn } from 'bun:test'

import {
  fixture,
  STYLE,
} from '../../../plugin-utils/src/__tests__/reservation-fixture'
import { createCompiledDependencies } from '../compiled-dependencies'
import { createViteReservation } from '../reservation'
import { createServerWatch } from '../server-watch'

function setup(root: string) {
  const reservation = createViteReservation({
    package: '@devup-ui/react',
    include: [],
    importAliases: {},
    singleCss: true,
    cssDir: join(root, 'df/css'),
    distDir: join(root, 'df'),
  })
  reservation.configure({
    root,
    configFileDependencies: [join(root, 'devup.json')],
  })
  const batches: string[][] = []
  const watcher = Object.assign(new EventEmitter(), {
    add(paths: string[]) {
      batches.push(paths)
    },
  })
  const compiled = createCompiledDependencies()
  return { reservation, batches, watcher, compiled }
}

it('submits no prior paths or empty batches when dirty preparation and config refresh repeat', async () => {
  // Given
  using project = fixture()
  project.write('src/A.js', `${STYLE}; import '../supplier.js'`)
  project.write('supplier.js', 'export const tone=1')
  using reserve = spyOn(wasm, 'reserveClassSlots').mockImplementation(() => {})
  const state = setup(project.root)
  const watching = createServerWatch(
    state.watcher,
    state.reservation,
    state.compiled.remove,
  )
  try {
    watching.add([
      join(project.root, 'devup.json'),
      join(project.root, 'tailwind.css'),
    ])
    await state.reservation.reserve()
    const initial = state.batches.flat()
    // When
    state.watcher.emit('all', 'change', join(project.root, 'supplier.js'))
    await state.reservation.reserve()
    watching.add([
      join(project.root, 'devup.json'),
      join(project.root, 'tailwind.css'),
    ])
    await state.reservation.reserve()
    // Then
    expect(state.batches.flat()).toEqual(initial)
    expect(initial.length).toBe(new Set(initial).size)
    expect(state.batches.every((batch) => batch.length > 0)).toBe(true)
    expect(reserve).toHaveBeenCalledTimes(2)
  } finally {
    watching.close()
  }
})

it('registers new supplier and configuration inputs once when the real inventory changes', async () => {
  // Given
  using project = fixture()
  project.write('src/A.js', STYLE)
  using reserve = spyOn(wasm, 'reserveClassSlots').mockImplementation(() => {})
  const state = setup(project.root)
  const watching = createServerWatch(
    state.watcher,
    state.reservation,
    state.compiled.remove,
  )
  try {
    await state.reservation.reserve()
    const before = state.batches.length
    project.write('src/new-supplier.js', 'export const tone=2')
    project.write('src/A.js', `${STYLE}; import './new-supplier.js'`)
    project.write('new-config.json', '{}')
    project.write('new-tailwind.css', '@theme { --color-brand: red; }')
    const config = join(project.root, 'new-config.json')
    const tailwind = join(project.root, 'new-tailwind.css')
    // When
    state.reservation.configure({
      root: project.root,
      configFileDependencies: [config],
    })
    watching.add([config, tailwind, config])
    await state.reservation.reserve()
    state.watcher.emit('all', 'change', config)
    await state.reservation.reserve()
    watching.add([config, tailwind])
    // Then
    expect(state.batches.slice(before).flat()).toEqual([
      config,
      tailwind,
      join(project.root, 'src/new-supplier.js').replaceAll('\\', '/'),
    ])
    expect(reserve).toHaveBeenCalledTimes(3)
  } finally {
    watching.close()
  }
})

it('replays committed inputs to a new server lifetime without retaining the old listener', async () => {
  // Given
  using project = fixture()
  project.write('src/A.js', STYLE)
  using reserve = spyOn(wasm, 'reserveClassSlots').mockImplementation(() => {})
  const state = setup(project.root)
  const first = createServerWatch(
    state.watcher,
    state.reservation,
    state.compiled.remove,
  )
  await state.reservation.reserve()
  first.close()
  const before = state.batches.length
  const next = Object.assign(new EventEmitter(), {
    add(paths: string[]) {
      nextBatches.push(paths)
    },
  })
  const nextBatches: string[][] = []
  // When
  const second = createServerWatch(
    next,
    state.reservation,
    state.compiled.remove,
  )
  try {
    state.watcher.emit('all', 'change', join(project.root, 'src/A.js'))
    await state.reservation.reserve()
    expect(reserve).toHaveBeenCalledTimes(1)
    next.emit('all', 'change', join(project.root, 'src/A.js'))
    await state.reservation.reserve()
    // Then
    expect(nextBatches).toEqual([[...state.reservation.dependencies]])
    expect(state.batches).toHaveLength(before)
    expect(reserve).toHaveBeenCalledTimes(2)
  } finally {
    second.close()
  }
})

it('removes compiled consumer edges only on an actual unlink event', () => {
  // Given
  using project = fixture()
  const state = setup(project.root)
  const supplier = join(project.root, 'supplier.js')
  state.compiled.update('client', 'A.js', [supplier])
  const watching = createServerWatch(
    state.watcher,
    state.reservation,
    state.compiled.remove,
  )
  try {
    state.watcher.emit('all', 'change', supplier)
    expect(state.compiled.consumers('client', supplier)).toEqual(['A.js'])
    // When
    state.watcher.emit('all', 'unlink', 'A.js')
    // Then
    expect(state.compiled.consumers('client', supplier)).toEqual([])
  } finally {
    watching.close()
  }
})
