import { mkdirSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { fixture } from '../../../plugin-utils/src/__tests__/reservation-fixture'
import { packageBoundary } from '../package-boundary'

it.each([
  ['package.json', '\uFEFF{"name":"app"}', true],
  ['node_modules/accepted/package.json', '{"name":"accepted"}', true],
  ['node_modules/@scope/accepted/package.json', '{"name":"accepted"}', true],
  ['node_modules/@scope/package.json', '{"name":"scope"}', false],
  ['node_modules/accepted/dist/package.json', '{"name":"dist"}', false],
  ['package.json', '{"name":"app","sideEffects":[7]}', false],
  ['package.json', '{"name":"app","sideEffects":["*.css"]}', true],
  ['package.json', '{"name":"app","sideEffects":false}', true],
  ['package.json', '{"name":7}', false],
] as const)(
  'classifies governing scope %s with %s',
  (path, manifest, expected) => {
    // Given
    using project = fixture()
    project.write(path, manifest)
    const directory = join(project.root, path.slice(0, -'package.json'.length))
    const boundary = packageBoundary()
    // When / Then
    expect(boundary(directory)).toBe(expected)
    expect(boundary(directory)).toBe(expected)
  },
)

it('skips a directory occupying the manifest path', () => {
  // Given
  using project = fixture()
  mkdirSync(join(project.root, 'package.json'))
  // When / Then
  expect(packageBoundary()(project.root)).toBe(false)
})
