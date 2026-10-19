import { join } from 'node:path'

import { expect, it } from 'bun:test'

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
