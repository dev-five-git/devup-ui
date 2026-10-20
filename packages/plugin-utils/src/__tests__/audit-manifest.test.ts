import assert from 'node:assert/strict'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { createModuleResolver } from '../import-graph'
import { ConfigLoadError } from '../load-config'
import { PackageManifestShapeError } from '../package-manifest'
import { collectReservationInputs } from '../reservation-discovery'
import { reservationSource } from '../reservation-source'
import { fixture } from './reservation-fixture'

const red =
  'import {css} from "@devup-ui/react"; export const cls=css({color:"red"});'

function manifestFailure(read: () => unknown, path: string): ConfigLoadError {
  let captured: ConfigLoadError | undefined
  assert.throws(read, (error: unknown) => {
    assert(error instanceof ConfigLoadError)
    expect(error.file).toBe(path)
    assert(error.cause instanceof PackageManifestShapeError)
    expect(error.cause).toBeInstanceOf(TypeError)
    expect(error.cause.manifestPath).toBe(path)
    captured = error
    return true
  })
  assert(captured)
  return captured
}

it('isolates an unused null manifest while retaining the ordinary reachable resolver error', () => {
  // Given
  using fs = fixture()
  fs.write('src/entry.js', red)
  fs.write('src/Unused.js', 'import "@devup-ui/broken";')
  fs.write('node_modules/@devup-ui/broken/package.json', 'null')
  const options = {
    roots: ['src'],
    cwd: fs.root,
    package: '@devup-ui/react',
    importAliases: {},
  }
  const resolver = createModuleResolver({ cwd: fs.root })
  manifestFailure(
    () => resolver('@devup-ui/broken', join(fs.root, 'src/entry.js')),
    join(fs.root, 'node_modules/@devup-ui/broken/package.json'),
  )
  // When / Then: scratch discovery must not publish an unreachable manifest error.
  expect(() => collectReservationInputs(options)).not.toThrow()
})

it.each(['null', '[]', '"text"', '1', 'true', 'false'])(
  'isolates manifest shape %s only at the explicit reservation boundary',
  (manifest) => {
    // Given
    using fs = fixture()
    fs.write('node_modules/@devup-ui/broken/package.json', manifest)
    fs.write('node_modules/@devup-ui/broken/index.js', red)
    const resolver = createModuleResolver({ cwd: fs.root })
    const read = () => resolver('@devup-ui/broken', 'src/entry.js')
    // When / Then
    const failure = manifestFailure(
      read,
      join(fs.root, 'node_modules/@devup-ui/broken/package.json'),
    )
    const rethrow = () => {
      throw failure
    }
    assert.throws(
      () => reservationSource(rethrow),
      (error) => error === failure,
    )
    assert.throws(
      () => reservationSource(rethrow, false),
      (error) => error === failure,
    )
    expect(reservationSource(read, true)).toBeUndefined()
  },
)

it('discovers a repaired manifest without hiding ordinary resolver failures', () => {
  // Given
  using fs = fixture()
  fs.write('src/entry.js', 'import "@devup-ui/broken";')
  fs.write('node_modules/@devup-ui/broken/package.json', 'null')
  fs.write('node_modules/@devup-ui/broken/index.js', red)
  const options = {
    roots: ['src'],
    cwd: fs.root,
    package: '@devup-ui/react',
    importAliases: {},
  }
  const resolver = createModuleResolver({ cwd: fs.root })
  expect(collectReservationInputs(options).inputs).toEqual([])
  manifestFailure(
    () => resolver('@devup-ui/broken', 'src/entry.js'),
    join(fs.root, 'node_modules/@devup-ui/broken/package.json'),
  )
  fs.write('node_modules/@devup-ui/broken/package.json', '{}')
  // When
  const repaired = collectReservationInputs(options)
  // Then
  expect(repaired.inputs.map(({ code }) => code)).toEqual([red])
  expect(resolver('@devup-ui/broken', 'src/entry.js')).toMatchObject({
    code: red,
  })
})

it.each([new TypeError('resolver logic failure'), new Error('logic failure')])(
  'preserves unrelated errors at the explicit manifest boundary: %s',
  (error) => {
    // Given
    const read = () => {
      throw error
    }
    // When / Then
    expect(() => reservationSource(read, true)).toThrow(error)
  },
)
