import { realpathSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { buildStaticImportGraph } from '../import-graph'
import { fixture } from './reservation-fixture'

const BROKEN = 'Package manifest must contain a JSON object'

function project() {
  const fs = fixture()
  const root = realpathSync(fs.root)
  fs.write('node_modules/@devup-ui/broken/package.json', 'null')
  fs.write('node_modules/@devup-ui/broken/index.js', 'export default 1')
  fs.write('src/a.js', "import './shared.js'")
  fs.write('src/b.js', "import './shared.js'")
  return Object.assign(fs, {
    path: (name: string) => join(root, name),
    graph: (entries?: string[]) =>
      buildStaticImportGraph(join(root, 'src'), undefined, {
        cwd: root,
        entries: entries?.map((name) => join(root, name)),
      }),
  })
}

it('builds the graph when only a file the entries cannot reach imports an unreadable manifest', () => {
  // Given: the entry reaches a shared file twice and an orphan imports the broken package.
  using fs = project()
  fs.write('src/entry.js', "import './a.js'; import './b.js'")
  fs.write('src/shared.js', 'export const shared = 1')
  fs.write('src/Unused.js', "import '@devup-ui/broken'")
  // When: the entries are known.
  const graph = fs.graph(['src/entry.js'])
  // Then: the orphan stays in the graph and its failure is not fatal.
  expect(graph.fileSet.has(fs.path('src/Unused.js'))).toBe(true)
  expect(graph.fileSet.has(fs.path('src/shared.js'))).toBe(true)
})

it('stops when the entries statically reach the file that imports an unreadable manifest', () => {
  // Given: a shared file reached through two importers imports the broken package.
  using fs = project()
  fs.write('src/entry.js', "import './a.js'; import './b.js'")
  fs.write('src/shared.js', "import '@devup-ui/broken'")
  // When / Then
  expect(() => fs.graph(['src/entry.js'])).toThrow(BROKEN)
})

it('stops when the entries reach the file only through a dynamic import', () => {
  // Given
  using fs = project()
  fs.write('src/entry.js', "import('./lazy.js')")
  fs.write('src/lazy.js', "import '@devup-ui/broken'")
  fs.write('src/shared.js', 'export const shared = 1')
  // When / Then
  expect(() => fs.graph(['src/entry.js'])).toThrow(BROKEN)
})

it('keeps stopping on an unreadable manifest of any file when no entries are known', () => {
  // Given
  using fs = project()
  fs.write('src/shared.js', 'export const shared = 1')
  fs.write('src/Unused.js', "import '@devup-ui/broken'")
  // When / Then
  expect(() => fs.graph()).toThrow(BROKEN)
})
