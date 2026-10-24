import { join } from 'node:path'

import { expect, it, mock } from 'bun:test'

import type { ReservationInventory } from '../reservation-discovery'
import { createSharedSlotReservation } from '../shared-slots'
import { fixture, flags, recorder, STYLE } from './reservation-fixture'

it('exposes committed suppliers without starting another discovery pass', () => {
  // Given
  using fs = fixture()
  fs.write('src/A.js', `${STYLE}; import '../outside/supplier.js'`)
  fs.write('outside/supplier.js', 'export const n=4')
  const manager = createSharedSlotReservation(recorder().engine)
  expect(manager.dependencies).toEqual([])
  // When
  manager.reserve(
    {
      roots: ['src'],
      cwd: fs.root,
      package: '@devup-ui/react',
      importAliases: {},
    },
    flags,
  )
  // Then
  expect(manager.dependencies).toEqual([
    join(fs.root, 'outside/supplier.js').replaceAll('\\', '/'),
    join(fs.root, 'src/A.js').replaceAll('\\', '/'),
  ])
})

it('installs current uncommitted metadata before engine entry and skips unchanged batches', () => {
  // Given
  using fs = fixture()
  fs.write('src/A.js', `${STYLE}; import '../supplier.js'`)
  fs.write('supplier.js', 'export const n=4')
  const events: string[] = []
  const manager = createSharedSlotReservation({
    reserveClassSlots() {
      events.push('engine')
    },
  })
  const discovery = {
    roots: ['src'],
    cwd: fs.root,
    package: '@devup-ui/react',
    importAliases: {},
  }
  const beforeReserve = mock((inventory: ReservationInventory) => {
    expect(manager.dependencies).toEqual([])
    expect(
      inventory.sources.get(join(fs.root, 'supplier.js').replaceAll('\\', '/')),
    ).toBe('export const n=4')
    expect(inventory.resolutions.imports.size).toBe(2)
    events.push('metadata')
  })
  // When
  manager.reserve(discovery, { ...flags, beforeReserve })
  manager.reserve(discovery, { ...flags, beforeReserve })
  // Then
  expect(events).toEqual(['metadata', 'engine'])
  expect(beforeReserve).toHaveBeenCalledTimes(1)
})

it('keeps failed callback batches dirty and retries without publishing dependencies', () => {
  // Given
  using fs = fixture()
  fs.write('src/A.js', STYLE)
  const recording = recorder()
  const manager = createSharedSlotReservation(recording.engine)
  const discovery = {
    roots: ['src'],
    cwd: fs.root,
    package: '@devup-ui/react',
    importAliases: {},
  }
  const beforeReserve = mock((): void => {
    throw new TypeError('metadata failure')
  })
  // When
  expect(() => manager.reserve(discovery, { ...flags, beforeReserve })).toThrow(
    TypeError,
  )
  // Then
  expect(recording.packets).toEqual([])
  expect(manager.dependencies).toEqual([])
  beforeReserve.mockImplementation(() => {})
  expect(manager.reserve(discovery, { ...flags, beforeReserve })).toBe(true)
  expect(recording.packets).toHaveLength(1)
})
