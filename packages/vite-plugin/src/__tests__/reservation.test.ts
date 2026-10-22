import * as fs from 'node:fs'
import { join } from 'node:path'

import * as utils from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import { expect, it, spyOn } from 'bun:test'

import {
  fixture,
  STYLE,
} from '../../../plugin-utils/src/__tests__/reservation-fixture'
import * as htmlReservation from '../html-reservation'
import { createViteReservation } from '../reservation'

function reservation(root: string) {
  return createViteReservation({
    package: '@devup-ui/react',
    include: [],
    importAliases: {},
    singleCss: true,
    cssDir: join(root, 'df/css'),
    distDir: join(root, 'df'),
  })
}

it('does no fresh inventory walk when concurrent or subsequent transforms see a clean revision', async () => {
  // Given
  using project = fixture()
  project.write('src/A.js', STYLE)
  using reserve = spyOn(wasm, 'reserveClassSlots').mockImplementation(() => {})
  const slots = reservation(project.root)
  slots.configure({ root: project.root })
  using walk = spyOn(fs, 'readdirSync')
  // When
  const first = slots.reserve()
  expect(slots.reserve()).toBe(first)
  await first
  const walks = walk.mock.calls.length
  await slots.reserve()
  await slots.reserve()
  // Then
  expect(walks).toBeGreaterThan(0)
  expect(walk.mock.calls).toHaveLength(walks)
  expect(reserve).toHaveBeenCalledTimes(1)
})

it('refreshes supplier-only changes once and coalesces sibling native HMR timestamps', async () => {
  // Given
  using project = fixture()
  project.write('src/A.js', `${STYLE}; import '../outside/supplier.js'`)
  project.write('outside/supplier.js', 'export const n=1')
  using reserve = spyOn(wasm, 'reserveClassSlots').mockImplementation(() => {})
  const slots = reservation(project.root)
  slots.configure({ root: project.root })
  const watched = new Set<string>()
  slots.watch((paths) => {
    for (const path of paths) watched.add(path)
  })
  await slots.reserve()
  // When
  project.write('outside/supplier.js', 'export const n=2')
  slots.invalidate(join(project.root, 'outside/supplier.js'), 10)
  await slots.reserve()
  slots.invalidate(join(project.root, 'outside/supplier.js'), 10)
  await slots.reserve()
  // Then
  expect(
    watched.has(
      join(project.root, 'outside/supplier.js').replaceAll('\\', '/'),
    ),
  ).toBe(true)
  expect(slots.dependencies).toContain(
    join(project.root, 'outside/supplier.js').replaceAll('\\', '/'),
  )
  expect(reserve).toHaveBeenCalledTimes(2)
  expect(
    reserve.mock.calls
      .at(-1)?.[0]
      .some((row: utils.ReservationRow) => row[1] === 'export const n=2'),
  ).toBe(true)
})

it('unions later default client HTML with server library and mixed explicit inputs before the first reservation', async () => {
  // Given
  using project = fixture()
  for (const name of ['A', 'B', 'C', 'D'])
    project.write(`outside/${name}.js`, STYLE)
  project.write('index.html', '<script type=module src=/outside/A.js></script>')
  project.write(
    'pages/custom.html',
    '<script type=module>import "../outside/B.js"</script>',
  )
  using reserve = spyOn(wasm, 'reserveClassSlots').mockImplementation(() => {})
  const slots = reservation(project.root)
  slots.configure({
    root: project.root,
    environments: {
      rsc: { build: { ssr: 'outside/D.js' } },
      client: {},
      mixed: { input: ['pages/custom.html', 'outside/C.js'] },
    },
  })
  slots.configure({
    root: project.root,
    environments: {
      rsc: { build: { lib: { entry: 'outside/D.js', formats: ['es'] } } },
    },
  })
  // When
  await slots.reserve()
  // Then
  expect(
    reserve.mock.calls[0]?.[0].map((row: utils.ReservationRow) => row[0]),
  ).toEqual(
    ['A', 'B', 'C', 'D'].map((name) =>
      join(project.root, `outside/${name}.js`).replaceAll('\\', '/'),
    ),
  )
})

it('discards a stale async HTML candidate and shares the refreshed preparation with every waiter', async () => {
  // Given
  using project = fixture()
  project.write('outside/A.js', STYLE)
  project.write('outside/B.js', STYLE)
  project.write('index.html', '<script type=module src=/outside/A.js></script>')
  using reserve = spyOn(wasm, 'reserveClassSlots').mockImplementation(() => {})
  const slots = reservation(project.root)
  slots.configure({ root: project.root })
  const gate = Promise.withResolvers<void>()
  const parse = htmlReservation.htmlReservationEntries
  using parser = spyOn(
    htmlReservation,
    'htmlReservationEntries',
  ).mockImplementationOnce(async (html, options) => {
    await gate.promise
    return parse(html, options)
  })
  // When
  const pending = slots.reserve()
  project.write('index.html', '<script type=module src=/outside/B.js></script>')
  slots.invalidate(join(project.root, 'index.html'))
  const sibling = slots.reserve()
  gate.resolve()
  await Promise.all([pending, sibling])
  // Then
  expect(parser).toHaveBeenCalledTimes(2)
  expect(reserve).toHaveBeenCalledTimes(1)
  expect(
    reserve.mock.calls[0]?.[0].map((row: utils.ReservationRow) => row[0]),
  ).toEqual([join(project.root, 'outside/B.js').replaceAll('\\', '/')])
})

it('ignores Devup-generated outputs but refreshes configuration and resolution events', async () => {
  // Given
  using project = fixture()
  project.write('src/A.js', STYLE)
  using reserve = spyOn(wasm, 'reserveClassSlots').mockImplementation(() => {})
  const slots = reservation(project.root)
  slots.configure({ root: project.root })
  await slots.reserve()
  // When
  slots.invalidate(join(project.root, 'df/css/devup-ui.css'), 1)
  slots.invalidate(join(project.root, 'df/theme.d.ts'), 2)
  await slots.reserve()
  expect(reserve).toHaveBeenCalledTimes(1)
  slots.invalidate(join(project.root, 'tsconfig.json'), 3)
  await slots.reserve()
  slots.configure({ root: project.root, input: 'src/A.js' })
  await slots.reserve()
  // Then
  expect(reserve).toHaveBeenCalledTimes(3)
})

it('propagates preparation errors and retries the same dirty revision without publishing it clean', async () => {
  // Given
  using project = fixture()
  project.write('src/A.js', STYLE)
  using reserve = spyOn(wasm, 'reserveClassSlots').mockImplementationOnce(
    () => {
      throw new TypeError('engine failure')
    },
  )
  const slots = reservation(project.root)
  slots.configure(undefined)
  await slots.reserve()
  slots.configure({ root: project.root })
  // When / Then
  await expect(slots.reserve()).rejects.toThrow(TypeError)
  reserve.mockImplementation(() => {})
  await slots.reserve()
  expect(reserve).toHaveBeenCalledTimes(2)
})
