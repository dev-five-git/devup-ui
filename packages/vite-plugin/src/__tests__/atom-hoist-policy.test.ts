import { join } from 'node:path'

import * as utils from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import { expect, it, spyOn } from 'bun:test'

import {
  fixture,
  STYLE,
} from '../../../plugin-utils/src/__tests__/reservation-fixture'
import { createViteReservation } from '../reservation'

it.each([undefined, 0, -1, NaN, Infinity, -Infinity, 0.5, 1, 2, 3])(
  'installs the numeric policy for %s before scratch, ignoring it in singleCss',
  async (atomHoist) => {
    // Given
    using project = fixture()
    for (const name of ['A', 'B'])
      project.write(`physical/${name}.js`, `${STYLE}; import '../shared.js'`)
    project.write('shared.js', STYLE)
    for (const singleCss of [false, true]) {
      using canonical = spyOn(wasm, 'importCanonicalMap').mockImplementation(
        () => {},
      )
      using routes = spyOn(wasm, 'importFileRoutes').mockImplementation(
        () => {},
      )
      using threshold = spyOn(wasm, 'setAtomHoist').mockImplementation(() => {})
      using scan = spyOn(utils, 'buildStaticImportGraph')
      const enabled =
        !singleCss &&
        (atomHoist === undefined ||
          (Number.isFinite(atomHoist) && atomHoist > 0))
      using engine = spyOn(wasm, 'reserveClassSlots').mockImplementation(() => {
        expect(canonical).toHaveBeenCalledTimes(enabled ? 1 : 0)
        expect(threshold).toHaveBeenCalledTimes(enabled ? 1 : 0)
      })
      const slots = createViteReservation({
        package: '@devup-ui/react',
        include: [],
        importAliases: {},
        singleCss,
        ...(atomHoist === undefined ? {} : { atomHoist }),
        cssDir: join(project.root, 'df/css'),
        distDir: join(project.root, 'df'),
      })
      slots.configure({
        root: project.root,
        input: ['physical/A.js', 'physical/B.js'],
      })
      // When
      await slots.reserve()
      // Then
      expect(engine).toHaveBeenCalledTimes(1)
      expect(scan).not.toHaveBeenCalled()
      if (enabled) {
        expect(threshold).toHaveBeenCalledWith(Math.max(2, atomHoist ?? 2))
        expect(routes.mock.calls[0]?.[0]).toMatchObject({
          [join(project.root, 'shared.js').replaceAll('\\', '/')]: [0, 1],
        })
      } else expect(routes).not.toHaveBeenCalled()
      expect(engine.mock.calls[0]?.slice(2, 5)).toEqual([
        singleCss,
        true,
        false,
      ])
    }
  },
)

it('folds canonical reach and skips global buckets before installing the threshold', async () => {
  // Given
  using project = fixture()
  project.write('src/A.js', STYLE)
  using _canonicalPlan = spyOn(utils, 'buildCanonicalMap').mockReturnValue({
    child: 'parent',
    global: '@global',
  })
  using _reachPlan = spyOn(utils, 'computeFileReach').mockReturnValue({
    child: [0],
    parent: [0, 1],
    global: [0, 1],
  })
  const events: string[] = []
  using _canonical = spyOn(wasm, 'importCanonicalMap').mockImplementation(
    () => {
      events.push('canonical')
    },
  )
  using routes = spyOn(wasm, 'importFileRoutes').mockImplementation(() => {
    events.push('routes')
  })
  using _threshold = spyOn(wasm, 'setAtomHoist').mockImplementation(() => {
    events.push('threshold')
  })
  using _engine = spyOn(wasm, 'reserveClassSlots').mockImplementation(() => {
    events.push('scratch')
  })
  const slots = createViteReservation({
    package: '@devup-ui/react',
    include: [],
    importAliases: {},
    singleCss: false,
    cssDir: join(project.root, 'df/css'),
    distDir: join(project.root, 'df'),
  })
  slots.configure({ root: project.root })
  // When
  await slots.reserve()
  // Then
  expect(routes).toHaveBeenCalledWith({ parent: [0, 1] })
  expect(events).toEqual(['canonical', 'routes', 'threshold', 'scratch'])
})
