import { join } from 'node:path'

import { expect, it, mock } from 'bun:test'

import { collectReservationInputs } from '../reservation-discovery'
import { createSharedSlotReservation } from '../shared-slots'
import { fixture, flags, recorder, STYLE } from './reservation-fixture'

it.each(['import', 'entry', 'broken'] as const)(
  'records actual %s package metadata independently of source inventory and nearer dist scopes',
  (kind) => {
    // Given
    using project = fixture()
    project.write(
      'src/A.js',
      kind === 'entry' ? STYLE : `${STYLE}; import 'accepted'`,
    )
    project.write(
      'node_modules/accepted/package.json',
      kind === 'broken' ? '{' : '{"exports":"./dist/red.js"}',
    )
    project.write(
      'node_modules/accepted/dist/package.json',
      '{"name":"nearer"}',
    )
    project.write(
      'node_modules/accepted/dist/red.js',
      'export const tone="red"',
    )
    const options = {
      roots: ['src'],
      cwd: project.root,
      package: '@devup-ui/react',
      importAliases: {},
      include: ['accepted'],
      ...(kind === 'entry'
        ? { entries: [{ request: 'accepted', context: project.root }] }
        : {}),
    }
    // When
    const inventory = collectReservationInputs(options)
    // Then
    const metadata = join(
      project.root,
      'node_modules/accepted/package.json',
    ).replaceAll('\\', '/')
    expect(inventory.packageManifests).toEqual([metadata])
    expect(inventory.dependencies).not.toContain(metadata)
    expect(inventory.sources.has(metadata)).toBe(false)
  },
)

it('refreshes metadata provenance when equivalent entry resolutions change the selected manifest', () => {
  // Given
  using project = fixture()
  project.write('src/A.js', STYLE)
  project.write(
    'node_modules/accepted/package.json',
    '{"main":"../../src/A.js"}',
  )
  const manager = createSharedSlotReservation(recorder().engine)
  const options = {
    roots: ['src'],
    cwd: project.root,
    package: '@devup-ui/react',
    importAliases: {},
  }
  manager.reserve(
    { ...options, entries: [{ request: './src/A.js', context: project.root }] },
    flags,
  )
  // When
  manager.reserve(
    { ...options, entries: [{ request: 'accepted', context: project.root }] },
    flags,
  )
  // Then
  expect(manager.packageManifests).toEqual([
    join(project.root, 'node_modules/accepted/package.json').replaceAll(
      '\\',
      '/',
    ),
  ])
})

it.each(['callback', 'engine'] as const)(
  'does not publish package metadata when %s fails',
  (failure) => {
    // Given
    using project = fixture()
    project.write('src/A.js', `${STYLE}; import 'accepted'`)
    project.write('node_modules/accepted/package.json', '{"main":"index.js"}')
    project.write('node_modules/accepted/index.js', 'export const n=1')
    const fail = mock(() => {
      throw new TypeError('preparation failed')
    })
    const manager = createSharedSlotReservation(
      failure === 'engine' ? { reserveClassSlots: fail } : recorder().engine,
    )
    const options = {
      roots: ['src'],
      cwd: project.root,
      package: '@devup-ui/react',
      importAliases: {},
      include: ['accepted'],
    }
    // When
    expect(() =>
      manager.reserve(options, {
        ...flags,
        ...(failure === 'callback' ? { beforeReserve: fail } : {}),
      }),
    ).toThrow(TypeError)
    // Then
    expect(manager.packageManifests).toEqual([])
    expect(manager.dependencies).toEqual([])
  },
)
