import { join } from 'node:path'

import type { ReservationRow } from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import { expect, it, spyOn } from 'bun:test'

import {
  fixture,
  STYLE,
} from '../../../plugin-utils/src/__tests__/reservation-fixture'
import { createViteReservation } from '../reservation'

function reservation(root: string, location: 'root' | 'nested') {
  return createViteReservation({
    package: '@devup-ui/react',
    include: [],
    importAliases: {},
    singleCss: true,
    cssDir: location === 'root' ? root : join(root, 'df/css'),
    distDir: join(root, 'df'),
  })
}

it.each(['root', 'nested'] as const)(
  'ignores only owned generated files when cssDir is %s',
  async (location) => {
    // Given
    using project = fixture()
    project.write('src/Initial.js', STYLE)
    using reserve = spyOn(wasm, 'reserveClassSlots').mockImplementation(
      () => {},
    )
    const slots = reservation(project.root, location)
    slots.configure({ root: project.root })
    await slots.reserve()
    const cssDir =
      location === 'root' ? project.root : join(project.root, 'df/css')
    // When
    for (const path of [
      join(cssDir, 'devup-ui.css'),
      join(cssDir, 'devup-ui-12.css'),
      join(project.root, 'df/compat.d.ts'),
      join(project.root, 'df/theme.d.ts'),
      join(project.root, 'df/.gitignore'),
    ])
      slots.invalidate(path, 1)
    await slots.reserve()
    // Then
    expect(reserve).toHaveBeenCalledTimes(1)
  },
)

it.each(['root', 'nested'] as const)(
  'includes newly created TSX and coalesces sibling timestamps when cssDir is %s',
  async (location) => {
    // Given
    using project = fixture()
    project.write('src/Initial.js', STYLE)
    using reserve = spyOn(wasm, 'reserveClassSlots').mockImplementation(
      () => {},
    )
    const slots = reservation(project.root, location)
    slots.configure({ root: project.root })
    await slots.reserve()
    project.write('src/New.tsx', STYLE)
    // When
    slots.invalidate(join(project.root, 'src/New.tsx'), 2)
    await slots.reserve()
    slots.invalidate(join(project.root, 'src/New.tsx'), 2)
    await slots.reserve()
    // Then
    expect(reserve).toHaveBeenCalledTimes(2)
    expect(
      reserve.mock.calls.at(-1)?.[0].map((row: ReservationRow) => row[0]),
    ).toContain(join(project.root, 'src/New.tsx').replaceAll('\\', '/'))
  },
)

it.each([
  'df/Entry.js',
  'df/New.tsx',
  'df/tsconfig.json',
  'devup.json',
  'src/devup-ui.css',
  'df/nested/theme.d.ts',
  'df/css/other.css',
])(
  'preserves physical events for %s inside output-directory labels',
  async (path) => {
    // Given
    using project = fixture()
    project.write('df/Entry.js', `${STYLE}; import './supplier.js'`)
    project.write('df/supplier.js', 'export const tone="red"')
    using reserve = spyOn(wasm, 'reserveClassSlots').mockImplementation(
      () => {},
    )
    const slots = reservation(project.root, 'root')
    slots.configure({ root: project.root, input: 'df/Entry.js' })
    await slots.reserve()
    // When
    slots.invalidate(join(project.root, path), 3)
    await slots.reserve()
    // Then
    expect(reserve).toHaveBeenCalledTimes(2)
  },
)

it('refreshes a real supplier under distDir without changing its exact source entry', async () => {
  // Given
  using project = fixture()
  project.write('df/Entry.js', `${STYLE}; import './supplier.js'`)
  project.write('df/supplier.js', 'export const tone="red"')
  using reserve = spyOn(wasm, 'reserveClassSlots').mockImplementation(() => {})
  const slots = reservation(project.root, 'nested')
  slots.configure({ root: project.root, input: 'df/Entry.js' })
  await slots.reserve()
  // When
  project.write('df/supplier.js', 'export const tone="blue"')
  slots.invalidate(join(project.root, 'df/supplier.js'), 4)
  await slots.reserve()
  // Then
  expect(
    reserve.mock.calls
      .at(-1)?.[0]
      .some((row: ReservationRow) => row[1] === 'export const tone="blue"'),
  ).toBe(true)
})
