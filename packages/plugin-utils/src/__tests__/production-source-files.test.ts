import * as fs from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { afterEach, beforeEach, expect, it, spyOn } from 'bun:test'

import { collectNumberedFiles } from '../numbering'
import { enumerateProductionSourceFiles } from '../production-source-files'

let root: string
beforeEach(() => {
  const parent = join(tmpdir(), 'opencode/workers/w20-plugins-core')
  fs.mkdirSync(parent, { recursive: true })
  root = fs.realpathSync.native(fs.mkdtempSync(join(parent, 'w20k-p1-')))
})
afterEach(() => fs.rmSync(root, { recursive: true, force: true }))
function file(path: string, content = 'export {}'): string {
  const target = join(root, path)
  fs.mkdirSync(dirname(target), { recursive: true })
  fs.writeFileSync(target, content)
  return target
}
function inventory(paths: readonly string[]) {
  return [...paths].sort().map((path) => ({ path, realPath: path }))
}

it('demonstrates physical omissions when the unchanged legacy collector filters source', () => {
  // Given physical, test-named and linked extraction candidates.
  file('src/plain.ts')
  file('src/view.test.tsx', "import '@devup-ui/react'")
  const target = file('outside/value.ts', "import '@devup-ui/react'")
  fs.symlinkSync(dirname(target), join(root, 'src/linked'), 'junction')
  // When legacy numbering uses its existing scanner and raw needles.
  const legacy = collectNumberedFiles({
    roots: [join(root, 'src')],
    needles: ['@devup-ui/react'],
  })
  // Then all three real candidates are absent; legacy behavior is untouched.
  expect(legacy).toEqual([])
})

it('enumerates needle-free and test-named physical files when production discovery runs', () => {
  // Given files whose eligibility must not depend on styling text or test names.
  const plain = file('src/plain.ts')
  const test = file('src/view.test.tsx')
  // When the new dormant enumerator visits the source root.
  const files = enumerateProductionSourceFiles({ roots: [join(root, 'src')] })
  // Then each physical source retains its lexical extraction path.
  expect(files).toEqual(inventory([plain, test]))
})

it.each([false, true])('keeps input permutations equal (%s)', (reverse) => {
  // Given overlapping project roots, outside entries and explicit sources.
  const paths = ['p/Z.spec.ts', 'p/a.ts', 'e/b.ts', 's/c.ts'].map((path) =>
    file(path),
  )
  const entry = file('e/index.html')
  file('s/unselected.txt')
  file('s/unselected.ts')
  const roots = ['p', 'p/nested', 'missing']
  const sources = ['s/c.ts', 'p/a.ts']
  const entries = [entry, join(root, 'p/a.ts')]
  // When each configuration list arrives in either order.
  const files = enumerateProductionSourceFiles({
    cwd: root,
    roots: reverse ? roots.toReversed() : roots,
    sourceFiles: reverse ? sources.toReversed() : sources,
    entries: reverse ? entries.toReversed() : entries,
  })
  // Then exact paths are deduplicated and codepoint-sorted, not locale-sorted.
  expect(files).toEqual(inventory(paths))
})

it.each([false, true])('retains aliases despite cycles (%s)', (reverse) => {
  // Given two aliases, an ancestry cycle and physical files in node_modules.
  const target = file('node_modules/pkg/dist/value.ts')
  fs.mkdirSync(join(root, 'src'))
  fs.symlinkSync(dirname(target), join(root, 'src/first'), 'junction')
  fs.symlinkSync(dirname(target), join(root, 'src/second'), 'junction')
  const cycle = join(dirname(target), 'cycle')
  fs.symlinkSync(join(root, 'src'), cycle, 'junction')
  const roots = ['src', 'src/first']
  // When project and explicit linked roots are traversed without global realpath dedupe.
  const files = enumerateProductionSourceFiles({
    cwd: root,
    roots: reverse ? roots.toReversed() : roots,
  })
  // Then both reachable lexical spellings survive and traversal terminates.
  expect(files).toEqual(
    ['first', 'second'].map((name) => ({
      path: join(root, `src/${name}/value.ts`),
      realPath: target,
    })),
  )
})

it('keeps linked source files when an explicit lexical file is selected', () => {
  // Given a file alias (native file symlink on hosts permitted to create it).
  const target = file('physical/value.ts')
  const alias = join(root, 'alias.ts')
  if (process.platform === 'win32') fs.linkSync(target, alias)
  else fs.symlinkSync(target, alias, 'file')
  // When only that lexical source is selected.
  const files = enumerateProductionSourceFiles({
    roots: [],
    sourceFiles: [alias],
  })
  // Then the extraction spelling remains the alias, with native physical identity.
  expect(files).toEqual([
    { path: alias, realPath: fs.realpathSync.native(alias) },
  ])
})

it('keeps installed dist when only nested own output is excluded', () => {
  // Given same-basename package dist and own output plus blanket dependencies.
  const kept = file('node_modules/pkg/dist/kept.js')
  const selected = file('node_modules/other/selected.ts')
  const own = file('build/nested/dist/poison.ts')
  file('node_modules/unused/absent.ts')
  file('src/nested/node_modules/unused/absent.ts')
  // When a project root, explicit package root and source are selected.
  const files = enumerateProductionSourceFiles({
    roots: [root, dirname(kept)],
    sourceFiles: [selected],
    exclude: [dirname(own)],
  })
  // Then absolute output exclusion does not remove installed package dist.
  expect(files).toEqual(inventory([kept, selected]))
})

it('excludes original and physical directories before any directory read', () => {
  // Given excluded roots and linked physical targets, with a retained sibling.
  const kept = file('src/kept.ts')
  const blocked = file('blocked/poison.ts')
  file('src/generated/poison.ts')
  fs.symlinkSync(dirname(blocked), join(root, 'src/physical'), 'junction')
  fs.symlinkSync(dirname(kept), join(root, 'lexical'), 'junction')
  const read = spyOn(fs, 'readdirSync')
  try {
    // When exclusions apply independently to lexical and physical targets.
    const files = enumerateProductionSourceFiles({
      roots: [root, 'lexical', 'blocked'],
      cwd: root,
      exclude: ['generated', dirname(blocked), join(root, 'lexical')],
    })
    // Then excluded paths never reach directory IO, even through a link.
    expect(files).toEqual([{ path: kept, realPath: kept }])
    expect(read.mock.calls.map(([path]) => path)).toEqual([root, dirname(kept)])
  } finally {
    read.mockRestore()
  }
})

it('enumerates dependencies when node_modules itself is an explicit root', () => {
  // Given an explicitly supplied dependency tree and unrelated project source.
  const selected = file('node_modules/pkg/dist/value.ts')
  file('src/unselected.ts')
  // When node_modules is configured directly, not discovered by project descent.
  const files = enumerateProductionSourceFiles({
    cwd: root,
    roots: ['node_modules'],
  })
  // Then its selected package source survives the basename traversal rule.
  expect(files).toEqual(inventory([selected]))
})

it.each([[undefined], [false], [true], [[]], [['.MDOWN']]])(
  'selects exact extensions (%j)',
  (selection) => {
    // Given all eight JS/TS extensions and distinct Markdown candidates.
    const js = 'ts tsx mts cts js jsx mjs cjs'
      .split(' ')
      .map((extension) => file(`src/value.${extension.toUpperCase()}`))
    const mdx = file('src/guide.MDX')
    const mdown = file('src/guide.mdown')
    file('src/guide.md')
    // When the shared opt-in policy is passed unchanged.
    const files = enumerateProductionSourceFiles({
      roots: [join(root, 'src')],
      ...(selection === undefined ? {} : { includeMdx: selection }),
    })
    // Then boolean true selects only MDX; lists select only literal extensions.
    const extra =
      selection === true
        ? [mdx]
        : Array.isArray(selection) && selection.length
          ? [mdown]
          : []
    expect(files).toEqual(inventory([...js, ...extra]))
  },
)

it('does not read source contents when physical eligibility is enumerated', () => {
  // Given a source with arbitrary bytes, not a styling needle.
  const path = file('src/unread.ts', '\0arbitrary bytes')
  const read = spyOn(fs, 'readFileSync')
  try {
    // When enumeration only inspects physical metadata.
    const files = enumerateProductionSourceFiles({ roots: [dirname(path)] })
    // Then content IO is absent.
    expect(files).toEqual([{ path, realPath: path }])
    expect(read).not.toHaveBeenCalled()
  } finally {
    read.mockRestore()
  }
})

it.each(['sourceFiles', 'entries'] as const)(
  'rejects absent explicit %s',
  (kind) => {
    // Given a missing explicit source or entry, unlike an optional root.
    const path = join(root, 'missing.ts')
    // When explicit physical discovery is requested.
    const run = () =>
      enumerateProductionSourceFiles({ roots: [], [kind]: [path] })
    // Then the native IO failure is not a virtual-input or fallback result.
    expect(run).toThrow(/ENOENT/)
  },
)

it('throws a native resolution error when an existing link target is absent', () => {
  // Given an existing directory link whose target has been removed.
  const target = join(root, 'target')
  fs.mkdirSync(target)
  fs.symlinkSync(target, join(root, 'dangling'), 'junction')
  fs.rmdirSync(target)
  // When even an optional configured root resolves that existing link.
  const run = () =>
    enumerateProductionSourceFiles({ roots: [join(root, 'dangling')] })
  // Then the physical resolution failure propagates.
  expect(run).toThrow(/ENOENT/)
})

it('propagates directory read faults when an existing root cannot be read', () => {
  // Given an existing root and a deterministic OS read-fault seam.
  const fault = Object.assign(new Error('directory read denied'), {
    code: 'EACCES',
  })
  const read = spyOn(fs, 'readdirSync').mockImplementation(() => {
    throw fault
  })
  try {
    // When directory enumeration fails after physical resolution.
    const run = () => enumerateProductionSourceFiles({ roots: [root] })
    // Then the exact original failure survives without a catch/fallback wrapper.
    expect(run).toThrow(fault)
  } finally {
    read.mockRestore()
  }
})
