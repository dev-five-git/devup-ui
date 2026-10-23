import { rmSync, symlinkSync } from 'node:fs'
import { dirname, join } from 'node:path'

import * as wasm from '@devup-ui/wasm'
import { expect, it, spyOn } from 'bun:test'

import {
  fixture,
  STYLE,
} from '../../../plugin-utils/src/__tests__/reservation-fixture'
import { createViteReservation } from '../reservation'

function reservation(root: string) {
  const slots = createViteReservation({
    package: '@devup-ui/react',
    include: ['accepted'],
    importAliases: {},
    singleCss: true,
    cssDir: join(root, 'df/css'),
    distDir: join(root, 'df'),
  })
  slots.configure({ root, input: 'src/A.js' })
  return slots
}

it('bounds named scope candidates without dropping closer missing manifests or config inputs', async () => {
  // Given
  using project = fixture()
  project.write('package.json', '{"name":"watch-fixture"}')
  project.write('src/deep/A.js', STYLE)
  using _reserve = spyOn(wasm, 'reserveClassSlots').mockImplementation(() => {})
  const slots = reservation(project.root)
  // When
  await slots.reserve()
  // Then
  expect(slots.dependencies).toContain(
    join(project.root, 'src/deep/package.json'),
  )
  expect(slots.dependencies).toContain(join(project.root, 'src/package.json'))
  expect(slots.dependencies).toContain(join(project.root, 'package.json'))
  expect(slots.dependencies).toContain(join(project.root, 'tsconfig.json'))
  expect(slots.dependencies).not.toContain(
    join(dirname(project.root), 'package.json'),
  )
})

it.each(['{"type":"module"}', '{', 'null', '{"name":""}'])(
  'continues past a non-governing manifest %s',
  async (manifest) => {
    // Given
    using project = fixture()
    project.write('package.json', '{"name":"watch-fixture"}')
    project.write('src/package.json', manifest)
    project.write('src/A.js', STYLE)
    using _reserve = spyOn(wasm, 'reserveClassSlots').mockImplementation(
      () => {},
    )
    const slots = reservation(project.root)
    // When
    await slots.reserve()
    // Then
    expect(slots.dependencies).toContain(join(project.root, 'src/package.json'))
    expect(slots.dependencies).toContain(join(project.root, 'package.json'))
    expect(slots.dependencies).not.toContain(
      join(dirname(project.root), 'package.json'),
    )
  },
)

it('recomputes the boundary after closer creation, deletion and recreation', async () => {
  // Given
  using project = fixture()
  project.write('package.json', '{"name":"watch-fixture"}')
  project.write('src/A.js', STYLE)
  using _reserve = spyOn(wasm, 'reserveClassSlots').mockImplementation(() => {})
  const slots = reservation(project.root)
  await slots.reserve()
  // When / Then: closer creation remains observable.
  project.write('src/package.json', '{"name":"closer"}')
  slots.invalidate(join(project.root, 'src/package.json'))
  await slots.reserve()
  expect(slots.dependencies).not.toContain(
    join(dirname(project.root), 'package.json'),
  )
  rmSync(join(project.root, 'src/package.json'))
  rmSync(join(project.root, 'package.json'))
  slots.invalidate(join(project.root, 'package.json'))
  await slots.reserve()
  expect(slots.dependencies).toContain(
    join(dirname(project.root), 'package.json'),
  )
  project.write('package.json', '{"name":"watch-fixture"}')
  slots.invalidate(join(project.root, 'package.json'))
  await slots.reserve()
  expect(slots.dependencies).not.toContain(
    join(dirname(project.root), 'package.json'),
  )
})

it('retains linked package-root exports above a nearer named dist scope with the same source inventory', async () => {
  // Given
  using project = fixture()
  project.write('package.json', '{"name":"watch-fixture"}')
  project.write(
    'src/A.js',
    `${STYLE}; import 'accepted'; import '../workspace/accepted/dist/red.js'; import '../workspace/accepted/dist/blue.js'`,
  )
  project.write(
    'workspace/accepted/package.json',
    '{"name":"accepted","exports":"./dist/red.js"}',
  )
  project.write(
    'workspace/accepted/dist/package.json',
    '{"name":"nearer-dist"}',
  )
  for (const color of ['red', 'blue'])
    project.write(
      `workspace/accepted/dist/${color}.js`,
      `export const tone='${color}'`,
    )
  project.write('node_modules/.keep', '')
  symlinkSync(
    join(project.root, 'workspace/accepted'),
    join(project.root, 'node_modules/accepted'),
    'junction',
  )
  using reserve = spyOn(wasm, 'reserveClassSlots').mockImplementation(() => {})
  const slots = reservation(project.root)
  await slots.reserve()
  const rows = reserve.mock.calls[0]?.[0]
  // When
  project.write(
    'workspace/accepted/package.json',
    '{"name":"accepted","exports":"./dist/blue.js"}',
  )
  const metadata = join(
    project.root,
    'workspace/accepted/package.json',
  ).replaceAll('\\', '/')
  expect(slots.dependencies).toContain(metadata)
  slots.invalidate(metadata)
  await slots.reserve()
  // Then
  expect(reserve).toHaveBeenCalledTimes(2)
  expect(reserve.mock.calls[1]?.[0]).toEqual(rows)
  expect(slots.dependencies).not.toContain(
    join(dirname(project.root), 'package.json'),
  )
})
