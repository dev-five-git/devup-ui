import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { collectReservationInputs } from '../reservation-discovery'
import { createSharedSlotReservation } from '../shared-slots'
import { fixture, flags, recorder, STYLE } from './reservation-fixture'

function tokens() {
  const fs = fixture()
  fs.write('src/A.tsx', `${STYLE}\nimport '@devup-ui/tokens';`)
  fs.write(
    'node_modules/@devup-ui/tokens/red.js',
    `${STYLE}\nexport const color="red"`,
  )
  fs.write(
    'node_modules/@devup-ui/tokens/blue.js',
    `${STYLE}\nexport const color="blue"`,
  )
  const options = {
    roots: ['src', 'node_modules/@devup-ui/tokens'],
    cwd: fs.root,
    package: '@devup-ui/react',
    importAliases: {},
  }
  const target = (main: string) =>
    fs.write(
      'node_modules/@devup-ui/tokens/package.json',
      JSON.stringify({ main }),
    )
  target('red.js')
  return { ...fs, options, target }
}

it('skips changed manifest bytes when actual resolution stays unchanged', () => {
  // Given
  using fs = tokens()
  const recorded = recorder()
  const slots = createSharedSlotReservation(recorded.engine)
  slots.reserve(fs.options, flags)
  // When
  fs.write(
    'node_modules/@devup-ui/tokens/package.json',
    '{"main":"red.js","version":"2"}',
  )
  const repeated = slots.reserve(fs.options, flags)
  // Then
  expect(repeated).toBe(false)
  expect(recorded.packets).toHaveLength(1)
})

it('refreshes changed import targets with identical inventory then skips the repeated revision', () => {
  // Given
  using fs = tokens()
  const recorded = recorder()
  const slots = createSharedSlotReservation(recorded.engine)
  const initial = collectReservationInputs(fs.options)
  slots.reserve(fs.options, flags)
  fs.target('blue.js')
  // When
  const refreshed = slots.reserve(fs.options, flags)
  const repeated = slots.reserve(fs.options, flags)
  // Then
  const revised = collectReservationInputs(fs.options)
  expect(revised.inputs).toEqual(initial.inputs)
  expect(revised.sources).toEqual(initial.sources)
  expect([refreshed, repeated]).toEqual([true, false])
  expect(recorded.packets).toHaveLength(2)
  expect(recorded.packets[1]).toEqual(recorded.packets[0])
})

it('refreshes repaired metadata even when the target was already selected', () => {
  // Given
  using fs = tokens()
  fs.write('node_modules/@devup-ui/tokens/package.json', 'null')
  const recorded = recorder()
  const slots = createSharedSlotReservation(recorded.engine)
  slots.reserve(fs.options, flags)
  fs.target('red.js')
  // When
  const repaired = slots.reserve(fs.options, flags)
  // Then
  expect(repaired).toBe(true)
  expect(recorded.packets).toHaveLength(2)
  expect(recorded.packets[1]).toEqual(recorded.packets[0])
})

it('detects changed static entry targets even when both physical inputs stay selected', () => {
  // Given
  using fs = tokens()
  fs.write('src/A.tsx', STYLE)
  const options = {
    ...fs.options,
    entries: [{ request: '@devup-ui/tokens', context: fs.root }],
  }
  const recorded = recorder()
  const slots = createSharedSlotReservation(recorded.engine)
  const initial = collectReservationInputs(options)
  slots.reserve(options, flags)
  fs.target('blue.js')
  // When
  const refreshed = slots.reserve(options, flags)
  // Then
  const revised = collectReservationInputs(options)
  expect(revised.inputs).toEqual(initial.inputs)
  expect(revised.sources).toEqual(initial.sources)
  expect(refreshed).toBe(true)
  expect(recorded.packets).toHaveLength(2)
  expect(recorded.packets[1]).toEqual(recorded.packets[0])
})

it('skips reordered or equivalent static entries with the same canonical targets', () => {
  // Given
  using fs = tokens()
  const entry = { request: '@devup-ui/tokens', context: fs.root }
  const source = { request: './src/A.tsx', context: fs.root }
  const recorded = recorder()
  const slots = createSharedSlotReservation(recorded.engine)
  slots.reserve({ ...fs.options, entries: [entry, source] }, flags)
  // When
  const repeated = slots.reserve(
    {
      ...fs.options,
      entries: [
        source,
        {
          ...entry,
          request: join(fs.root, 'node_modules/@devup-ui/tokens/red.js'),
        },
        entry,
      ],
    },
    flags,
  )
  // Then
  expect(repeated).toBe(false)
  expect(recorded.packets).toHaveLength(1)
})

it('detects exchanged specifier targets even when the importer target set stays identical', () => {
  // Given
  using fs = tokens()
  fs.write(
    'src/A.tsx',
    `${STYLE}\nimport '@devup-ui/tokens/red'; import '@devup-ui/tokens/blue';`,
  )
  const manifest = 'node_modules/@devup-ui/tokens/package.json'
  fs.write(manifest, '{"exports":{"./red":"./red.js","./blue":"./blue.js"}}')
  const recorded = recorder()
  const slots = createSharedSlotReservation(recorded.engine)
  slots.reserve(fs.options, flags)
  fs.write(manifest, '{"exports":{"./red":"./blue.js","./blue":"./red.js"}}')
  // When
  const refreshed = slots.reserve(fs.options, flags)
  // Then
  expect(refreshed).toBe(true)
  expect(recorded.packets).toHaveLength(2)
  expect(recorded.packets[1]).toEqual(recorded.packets[0])
})
