import { rmSync } from 'node:fs'
import { join, resolve } from 'node:path'

import { resetBuildState } from '@devup-ui/wasm'
import { expect, it, spyOn } from 'bun:test'

import {
  fixture,
  STYLE,
} from '../../../plugin-utils/src/__tests__/reservation-fixture'
import { createCompiledDependencies } from '../compiled-dependencies'
import * as htmlReservation from '../html-reservation'
import { createViteReservation } from '../reservation'
import { createServerWatch } from '../server-watch'

function provenanceReservation(root: string) {
  return createViteReservation({
    package: '@devup-ui/react',
    include: [],
    importAliases: {},
    singleCss: true,
    cssDir: join(root, 'df/css'),
    distDir: join(root, 'df'),
  })
}

it('leaves actual revision unchanged when unrelated arbitrary JSON is admitted', async () => {
  // Given
  resetBuildState()
  using project = fixture()
  project.write('src/A.js', STYLE)
  const reservation = provenanceReservation(project.root)
  reservation.configure({ root: project.root })
  await reservation.reserve()
  const revision = reservation.revision
  // When
  reservation.invalidate(join(project.root, 'arbitrary.json'), 42)
  await reservation.reserve()
  // Then
  expect(reservation.revision).toBe(revision)
  expect(reservation.prepared).toBe(revision)
})

for (const request of ['pages/missing.html', 'external/data.json']) {
  it(`admits explicit ${request} before first preparation`, () => {
    // Given
    using project = fixture()
    const reservation = provenanceReservation(project.root)
    reservation.configure({ root: project.root, input: request })
    const revision = reservation.revision
    // When
    reservation.invalidate(resolve(project.root, request))
    // Then
    expect(reservation.revision).toBe(revision + 1)
  })
}

for (const event of ['addDir', 'unlinkDir']) {
  it(`dirties native membership when outside-root ${event} arrives`, () => {
    // Given
    using project = fixture()
    const reservation = provenanceReservation(project.root)
    let deliver: (event: string, file: string) => void = () => {}
    const watcher = {
      add() {},
      on(_event: 'all', listener: typeof deliver) {
        deliver = listener
      },
      off() {},
    }
    const watching = createServerWatch(watcher, reservation, () => {})
    const revision = reservation.revision
    // When
    deliver(event, resolve(project.root, '../outside-candidate'))
    // Then
    expect(reservation.revision).toBe(revision + 1)
    watching.close()
  })
}

for (const file of [
  'tsconfig.json',
  'package.json',
  'bun.lock',
  'config/explicit.json',
]) {
  it(`admits initial ${file} before published dependencies exist`, () => {
    // Given
    using project = fixture()
    const reservation = provenanceReservation(project.root)
    reservation.configure({
      root: project.root,
      configFileDependencies: ['config/explicit.json'],
    })
    const revision = reservation.revision
    // When
    reservation.invalidate(join(project.root, file), 13)
    reservation.invalidate(join(project.root, file), 13)
    // Then
    expect(reservation.revision).toBe(revision + 1)
  })
}

it('discards first HTML candidate when it changes during the real parser await', async () => {
  // Given
  resetBuildState()
  using project = fixture()
  project.write('outside/A.js', STYLE)
  project.write('outside/B.js', STYLE)
  project.write(
    'index.html',
    '<script type="module" src="/outside/A.js"></script>',
  )
  const reservation = provenanceReservation(project.root)
  reservation.configure({ root: project.root })
  const entered = Promise.withResolvers<void>()
  const released = Promise.withResolvers<void>()
  const parse = htmlReservation.htmlReservationEntries
  using parser = spyOn(
    htmlReservation,
    'htmlReservationEntries',
  ).mockImplementationOnce(async (...args) => {
    const result = await parse(...args)
    entered.resolve()
    await released.promise
    return result
  })
  const pending = reservation.reserve()
  await entered.promise
  // When
  project.write(
    'index.html',
    '<script type="module" src="/outside/B.js"></script>',
  )
  reservation.invalidate(join(project.root, 'index.html'))
  released.resolve()
  await pending
  // Then
  expect(parser).toHaveBeenCalledTimes(2)
  expect(reservation.dependencies.map((path) => resolve(path))).toContain(
    join(project.root, 'outside/B.js'),
  )
  expect(reservation.dependencies.map((path) => resolve(path))).not.toContain(
    join(project.root, 'outside/A.js'),
  )
})

it('discovers a missing explicit HTML entry when it is created after preparation', async () => {
  // Given
  resetBuildState()
  using project = fixture()
  project.write('outside/A.js', STYLE)
  const reservation = provenanceReservation(project.root)
  reservation.configure({ root: project.root, input: 'pages/entry.html' })
  await reservation.reserve()
  // When
  project.write(
    'pages/entry.html',
    '<script type="module" src="/outside/A.js"></script>',
  )
  reservation.invalidate(join(project.root, 'pages/entry.html'))
  await reservation.reserve()
  // Then
  expect(reservation.dependencies.map((path) => resolve(path))).toContain(
    join(project.root, 'outside/A.js'),
  )
})

for (const operation of ['create', 'delete', 'recreate']) {
  it(`admits closer missing manifest ${operation} from exact provenance`, async () => {
    // Given
    resetBuildState()
    using project = fixture()
    project.write('package.json', '{"name":"outer"}')
    project.write('src/deep/A.js', STYLE)
    const reservation = provenanceReservation(project.root)
    reservation.configure({ root: project.root })
    await reservation.reserve()
    const file = join(project.root, 'src/package.json')
    expect(reservation.dependencies.map((path) => resolve(path))).toContain(
      file,
    )
    if (operation !== 'create') {
      project.write('src/package.json', '{"name":"closer"}')
      reservation.invalidate(file)
      await reservation.reserve()
    }
    if (operation === 'recreate') {
      rmSync(file)
      reservation.invalidate(file)
      await reservation.reserve()
    }
    const revision = reservation.revision
    // When
    if (operation === 'delete') rmSync(file)
    else project.write('src/package.json', '{"name":"closer"}')
    reservation.invalidate(file)
    await reservation.reserve()
    // Then
    expect(reservation.revision).toBe(revision + 1)
    expect(reservation.prepared).toBe(revision + 1)
  })
}

for (const suffix of ['json', 'css', 'cjs']) {
  it(`admits cross-environment compiled ${suffix} supplier without blanket suffix admission`, () => {
    // Given
    using project = fixture()
    const compiled = createCompiledDependencies()
    const supplier = join(project.root, `outside/supplier.${suffix}`)
    compiled.update('rsc', 'consumer.tsx?server', [supplier])
    const reservation = createViteReservation({
      package: '@devup-ui/react',
      include: [],
      importAliases: {},
      singleCss: true,
      cssDir: join(project.root, 'df/css'),
      distDir: join(project.root, 'df'),
      relevant: compiled.hasSupplier,
    })
    const revision = reservation.revision
    // When
    reservation.invalidate(join(project.root, `arbitrary.${suffix}`), 9)
    expect(reservation.revision).toBe(revision)
    reservation.invalidate(supplier, 9)
    // Then
    expect(reservation.revision).toBe(revision + 1)
    compiled.remove('consumer.tsx')
    expect(compiled.hasSupplier(supplier)).toBe(false)
  })
}
