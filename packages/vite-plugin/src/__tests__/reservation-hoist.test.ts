import { rmSync } from 'node:fs'
import { join } from 'node:path'

import * as utils from '@devup-ui/plugin-utils'
import * as wasm from '@devup-ui/wasm'
import { expect, it, spyOn } from 'bun:test'

import {
  fixture,
  STYLE,
} from '../../../plugin-utils/src/__tests__/reservation-fixture'
import { createViteReservation } from '../reservation'
import type { ReservationEnvironment } from '../reservation-inputs'

function reservation(root: string) {
  return createViteReservation({
    package: '@devup-ui/react',
    include: [],
    importAliases: {},
    singleCss: false,
    cssDir: join(root, 'df/css'),
    distDir: join(root, 'df'),
  })
}

it.each([false, true])(
  'installs the complete environment union before first scratch with reversed order %s',
  async (reverse) => {
    // Given
    using project = fixture()
    project.write(
      'outside/A.js',
      `${STYLE}; import '../shared.js'; import('../lazy.js')`,
    )
    project.write('outside/B.js', `${STYLE}; import '../shared.js'`)
    project.write('shared.js', STYLE)
    project.write('lazy.js', "import './shared.js'")
    project.write('src/unused.js', "import('../unused-lazy.js')")
    project.write('unused-lazy.js', STYLE)
    project.write(
      'index.html',
      '<script type=module src=/outside/A.js></script>',
    )
    using routes = spyOn(wasm, 'importFileRoutes').mockImplementation(() => {})
    using _canonical = spyOn(wasm, 'importCanonicalMap').mockImplementation(
      () => {},
    )
    using threshold = spyOn(wasm, 'setAtomHoist').mockImplementation(() => {})
    using plan = spyOn(utils, 'reservationGraph')
    using engine = spyOn(wasm, 'reserveClassSlots').mockImplementation(() => {
      expect(routes.mock.calls[0]?.[0]).toMatchObject({
        [join(project.root, 'shared.js').replaceAll('\\', '/')]: [0, 1, 2],
      })
      expect(threshold).toHaveBeenCalledWith(2)
    })
    const slots = reservation(project.root)
    const environments: Readonly<Record<string, ReservationEnvironment>>[] = [
      { client: {} },
      { server: { build: { ssr: 'outside/B.js' } } },
    ]
    for (const environment of reverse
      ? environments.toReversed()
      : environments)
      slots.configure({ root: project.root, environments: environment })
    slots.configure({
      root: project.root,
      environments: { duplicate: { input: 'outside/A.js' } },
    })
    // When
    const first = slots.reserve()
    expect(slots.reserve()).toBe(first)
    await first
    await slots.reserve()
    project.write('outside/C.js', `${STYLE}; import '../shared.js'`)
    slots.configure({
      root: project.root,
      environments: { later: { input: 'outside/C.js' } },
    })
    await slots.reserve()
    // Then
    expect(plan).toHaveBeenCalledTimes(1)
    expect(engine).toHaveBeenCalledTimes(2)
    expect(routes).toHaveBeenCalledTimes(1)
    expect(routes.mock.calls[0]?.[0]).not.toHaveProperty(
      join(project.root, 'unused-lazy.js').replaceAll('\\', '/'),
    )
  },
)

it('discards stale async candidates before planning and retries engine failure without reinstallation', async () => {
  // Given
  using project = fixture()
  project.write('outside/A.js', STYLE)
  project.write('outside/B.js', STYLE)
  project.write('index.html', '<script type=module src=/outside/A.js></script>')
  const parse = utils.htmlReservationEntries
  const gate = Promise.withResolvers<void>()
  using _parser = spyOn(utils, 'htmlReservationEntries').mockImplementationOnce(
    async (html, options) => {
      await gate.promise
      return parse(html, options)
    },
  )
  using graph = spyOn(utils, 'reservationGraph')
  using _canonical = spyOn(wasm, 'importCanonicalMap').mockImplementation(
    () => {},
  )
  using routes = spyOn(wasm, 'importFileRoutes').mockImplementation(() => {})
  using threshold = spyOn(wasm, 'setAtomHoist').mockImplementation(() => {})
  using engine = spyOn(wasm, 'reserveClassSlots').mockImplementationOnce(() => {
    throw new TypeError('scratch failure')
  })
  const slots = reservation(project.root)
  slots.configure({ root: project.root })
  // When
  const first = slots.reserve()
  project.write(
    'index.html',
    '<script type=module src=/outside/A.js></script><script type=module src=/outside/B.js></script>',
  )
  slots.invalidate(join(project.root, 'index.html'))
  gate.resolve()
  await expect(first).rejects.toThrow(TypeError)
  engine.mockImplementation(() => {})
  await slots.reserve()
  // Then
  expect(graph).toHaveBeenCalledTimes(1)
  expect(threshold).toHaveBeenCalledTimes(1)
  expect(routes).toHaveBeenCalledTimes(1)
  expect(engine).toHaveBeenCalledTimes(2)
})

it.each(['empty', 'single'])(
  'latches the initial %s outcome instead of promoting watch-time names',
  async (initial) => {
    // Given
    using project = fixture()
    if (initial === 'single') project.write('src/A.js', STYLE)
    using graph = spyOn(utils, 'reservationGraph')
    using canonical = spyOn(wasm, 'importCanonicalMap').mockImplementation(
      () => {},
    )
    using routes = spyOn(wasm, 'importFileRoutes').mockImplementation(() => {})
    using threshold = spyOn(wasm, 'setAtomHoist').mockImplementation(() => {})
    using _engine = spyOn(wasm, 'reserveClassSlots').mockImplementation(
      () => {},
    )
    const slots = reservation(project.root)
    slots.configure({ root: project.root })
    await slots.reserve()
    // When
    project.write('src/A.js', STYLE)
    project.write('src/B.js', STYLE)
    slots.invalidate()
    await slots.reserve()
    // Then
    expect(graph).toHaveBeenCalledTimes(1)
    expect(canonical).toHaveBeenCalledTimes(1)
    expect(routes).not.toHaveBeenCalled()
    expect(threshold).not.toHaveBeenCalled()
  },
)

it('does not partially install metadata when candidate planning fails and retries the preparation', async () => {
  // Given
  using project = fixture()
  project.write('src/A.js', STYLE)
  using planner = spyOn(utils, 'planAtomHoist').mockImplementationOnce(() => {
    throw new TypeError('candidate failure')
  })
  using canonical = spyOn(wasm, 'importCanonicalMap').mockImplementation(
    () => {},
  )
  using engine = spyOn(wasm, 'reserveClassSlots').mockImplementation(() => {})
  const slots = reservation(project.root)
  slots.configure({ root: project.root })
  // When
  await expect(slots.reserve()).rejects.toThrow(TypeError)
  // Then
  expect(canonical).not.toHaveBeenCalled()
  expect(engine).not.toHaveBeenCalled()
  planner.mockRestore()
  await slots.reserve()
  expect(canonical).toHaveBeenCalledTimes(1)
})

it.each([false, true])(
  'retains real counter leaves and frozen placement through watch history in reverse %s',
  async (reverse) => {
    // Given
    using project = fixture()
    const source = (value: string) =>
      `import {css} from '@devup-ui/react'; export const cls=css({${value}})`
    const shared = source("color:'orchid'")
    const privateStyle = source('opacity:.25')
    project.write('A.js', "import './S.js'; import './P.js'")
    project.write('B.js', "import './S.js'")
    project.write('S.js', shared)
    project.write('P.js', privateStyle)
    wasm.resetBuildState()
    wasm.setDebug(true)
    const slots = reservation(project.root)
    slots.configure({ root: project.root, input: ['A.js', 'B.js'] })
    const extract = (file: string, code: string) =>
      wasm.codeExtract(
        join(project.root, file).replaceAll('\\', '/'),
        code,
        '@devup-ui/react',
        './df/css',
        false,
        true,
        false,
        {},
      )
    try {
      await slots.reserve()
      extract('S.js', shared).free()
      extract('P.js', privateStyle).free()
      const files = wasm.exportFileMap()
      const classes = wasm.exportClassMap()
      expect(wasm.getCss(null, false)).toContain('color:orchid')
      expect(wasm.getCss(null, false)).not.toContain('opacity:.25')
      // When
      for (const file of reverse ? ['D.js', 'C.js'] : ['C.js', 'D.js'])
        project.write(file, source("outlineColor:'navy'"))
      project.write('B.js', "import './S.js'; import './P.js'")
      slots.configure({
        root: project.root,
        input: ['A.js', 'B.js', 'C.js', 'D.js'],
      })
      await slots.reserve()
      extract('P.js', privateStyle).free()
      extract('C.js', source("outlineColor:'navy'")).free()
      project.write('S.js', source("color:'plum'"))
      slots.invalidate(join(project.root, 'S.js'))
      await slots.reserve()
      extract('S.js', source("color:'plum'")).free()
      rmSync(join(project.root, 'P.js'))
      slots.invalidate(join(project.root, 'P.js'))
      await slots.reserve()
      project.write('P.js', privateStyle)
      slots.invalidate(join(project.root, 'P.js'))
      await slots.reserve()
      extract('P.js', privateStyle).free()
      // Then
      expect(JSON.parse(wasm.exportFileMap())).toMatchObject(JSON.parse(files))
      expect(JSON.parse(wasm.exportClassMap())).toMatchObject(
        JSON.parse(classes),
      )
      expect(wasm.getCss(null, false)).toContain('color:plum')
      expect(wasm.getCss(null, false)).not.toContain('opacity:.25')
      expect(wasm.getCss(null, false)).not.toContain('outline-color:navy')
      expect(wasm.getUnreservedClassSlotCount()).toBe(0)
    } finally {
      wasm.resetBuildState()
      wasm.setDebug(true)
    }
  },
)
