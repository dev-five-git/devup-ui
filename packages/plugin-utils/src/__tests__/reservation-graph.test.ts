import * as fs from 'node:fs'
import { resolve } from 'node:path'

import { expect, it, spyOn } from 'bun:test'

import {
  buildCanonicalMap,
  computeFileReach,
  computeReachableFiles,
} from '../import-graph'
import { collectReservationInputs } from '../reservation-discovery'
import { reservationGraph } from '../reservation-graph'
import { fixture, STYLE } from './reservation-fixture'

it('projects the closed real inventory including ordinary importers, include, cycles and typed edges without I/O', () => {
  // Given
  using project = fixture()
  project.write(
    'root.js',
    `export * from './physical/A.js'; require('./physical/B.js'); import('./physical/B.js'); import type {T} from './missing.js'; const fake="import './fake.js'"`,
  )
  project.write(
    'physical/A.js',
    `${STYLE}; import '../shared.js'; import 'accepted'; import './cycle.js'`,
  )
  project.write(
    'physical/B.js',
    `${STYLE}; require('../shared.js'); import('../lazy.js')`,
  )
  project.write('physical/cycle.js', "import './A.js'")
  project.write('shared.js', STYLE)
  project.write('lazy.js', "import './shared.js'")
  project.write('src/unused.js', "import('../unused-lazy.js')")
  project.write('unused-lazy.js', STYLE)
  project.write('df/stale.js', STYLE)
  project.write(
    'node_modules/accepted/package.json',
    '{"name":"accepted","main":"dist/index.js"}',
  )
  project.write('node_modules/accepted/dist/index.js', STYLE)
  const inventory = collectReservationInputs({
    roots: ['src'],
    cwd: project.root,
    entries: [{ request: './root.js', context: project.root }],
    include: ['accepted'],
    package: '@devup-ui/react',
    importAliases: {},
  })
  const native = (path: string) => resolve(project.root, path)
  const key = (path: string) => native(path).replaceAll('\\', '/')
  expect(inventory.inputs.some((row) => row.filename === key('root.js'))).toBe(
    false,
  )
  using read = spyOn(fs, 'readFileSync')
  using walk = spyOn(fs, 'readdirSync')
  // When
  const graph = reservationGraph(inventory)
  // Then
  expect(read).not.toHaveBeenCalled()
  expect(walk).not.toHaveBeenCalled()
  expect(graph.staticImports.get(native('root.js'))).toEqual(
    new Set([native('physical/A.js'), native('physical/B.js')]),
  )
  expect(graph.dynamicImports.get(native('root.js'))).toEqual(
    new Set([native('physical/B.js')]),
  )
  expect(graph.staticImporters.get(native('shared.js'))).toEqual(
    new Set([
      native('lazy.js'),
      native('physical/A.js'),
      native('physical/B.js'),
    ]),
  )
  expect(graph.fileSet.has(native('df/stale.js'))).toBe(false)
  expect(graph.fileSet.has(native('node_modules/accepted/dist/index.js'))).toBe(
    true,
  )
  expect(graph.fileSet.has(native('missing.js'))).toBe(false)
  const options = {
    graph,
    srcDir: project.root,
    cwd: project.root,
    keyBy: 'absolute' as const,
  }
  const reached = computeReachableFiles({
    ...options,
    entries: [native('root.js')],
  })
  expect(reached).toContain(native('lazy.js'))
  expect(reached).not.toContain(native('unused-lazy.js'))
  const reach = computeFileReach({
    ...options,
    entries: [
      native('root.js'),
      ...reached.filter((file) => graph.dynamicTargets.has(file)),
    ],
  })
  expect(reach[key('shared.js')]).toEqual([0, 1, 2])
  expect(reach[key('unused-lazy.js')]).toBeUndefined()
  expect(
    buildCanonicalMap(options)[key('node_modules/accepted/dist/index.js')],
  ).toBe(key('physical/A.js'))
})

it('keeps literal escaped CRLF require edges while excluding dotted and quoted calls', () => {
  // Given
  using project = fixture()
  project.write(
    'entry.js',
    `const x=require(\r\n './suppl\\u0069er.js'\r\n); const quoted="require('./fake.js')"; object.require('./fake.js'); require(name)`,
  )
  project.write('supplier.js', STYLE)
  const inventory = collectReservationInputs({
    roots: [],
    cwd: project.root,
    entries: [{ request: './entry.js', context: project.root }],
    package: '@devup-ui/react',
    importAliases: {},
  })
  // When
  const graph = reservationGraph(inventory)
  // Then
  expect(graph.files).toEqual(
    ['entry.js', 'supplier.js'].map((path) => resolve(project.root, path)),
  )
  expect(graph.staticImports.get(resolve(project.root, 'entry.js'))).toEqual(
    new Set([resolve(project.root, 'supplier.js')]),
  )
})
