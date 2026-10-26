import { join } from 'node:path'

import {
  createSharedSlotReservation,
  getBuildOwnership,
} from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import { expect, it, spyOn } from 'bun:test'

import {
  fixture,
  flags,
  recorder,
  STYLE,
} from '../../../plugin-utils/src/__tests__/reservation-fixture'
import { createViteReservation } from '../reservation'

it('forwards the actual Vite reservation batch without publishing scratch CSS', async () => {
  expect(getBuildOwnership().running).toBe(0)
  wasm.resetBuildState()
  using project = fixture()
  project.write('src/A.tsx', STYLE)
  const captured = recorder()
  const { reserveClassSlots } = wasm
  const reserve = reserveClassSlots.bind(wasm)
  using observed = spyOn(wasm, 'reserveClassSlots').mockImplementation(
    (...args) => {
      captured.engine.reserveClassSlots(...args)
      reserve(...args)
    },
  )
  const reservation = createViteReservation({
    package: '@devup-ui/react',
    include: [],
    importAliases: {},
    singleCss: true,
    cssDir: join(project.root, 'df/css'),
    distDir: join(project.root, 'df'),
  })
  const sheet = wasm.exportSheet()
  reservation.configure({ root: project.root })
  await reservation.reserve()
  expect(observed).toHaveBeenCalledTimes(1)
  expect(captured.packets).toHaveLength(1)
  expect(captured.packets[0]).toEqual({
    rows: [
      [
        join(project.root, 'src/A.tsx').replaceAll('\\', '/'),
        STYLE,
        './../df/css',
      ],
    ],
    packageName: '@devup-ui/react',
    singleCss: true,
    importMainCssInCode: true,
    importMainCssInCss: false,
    aliases: {},
  })
  expect(wasm.exportSheet()).toBe(sheet)
})

it('preserves the batch contract when shared-slot CSS flags supply the directory', () => {
  expect(getBuildOwnership().running).toBe(0)
  wasm.resetBuildState()
  using project = fixture()
  project.write('src/A.tsx', STYLE)
  const captured = recorder()
  const { reserveClassSlots } = wasm
  const reserve = reserveClassSlots.bind(wasm)
  const reservation = createSharedSlotReservation({
    reserveClassSlots(...args) {
      captured.engine.reserveClassSlots(...args)
      reserve(...args)
    },
  })
  const sheet = wasm.exportSheet()
  reservation.reserve(
    {
      roots: [join(project.root, 'src')],
      entries: [],
      include: [],
      cwd: project.root,
      package: '@devup-ui/react',
      importAliases: {},
      toId: (path) => path.replaceAll('\\', '/'),
    },
    flags,
  )
  expect(captured.packets).toHaveLength(1)
  expect(captured.packets[0]).toEqual({
    rows: [
      [
        join(project.root, 'src/A.tsx').replaceAll('\\', '/'),
        STYLE,
        '../df/devup-ui',
      ],
    ],
    packageName: '@devup-ui/react',
    singleCss: true,
    importMainCssInCode: true,
    importMainCssInCss: false,
    aliases: {},
  })
  expect(wasm.exportSheet()).toBe(sheet)
})
