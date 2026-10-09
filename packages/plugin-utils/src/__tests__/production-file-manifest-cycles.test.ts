import {
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { expect, it } from 'bun:test'

import { compareCodePoints } from '../import-graph'
import type { ProductionFileManifestOptions } from '../production-file-manifest'
import { enumerateProductionSourceFiles } from '../production-source-files'

async function collectProductionFileManifest(
  options: ProductionFileManifestOptions,
) {
  return (
    await import('../production-file-manifest')
  ).collectProductionFileManifest(options)
}

function memberships(paths: readonly string[], context: string) {
  return paths
    .map((path) => ({
      context,
      path,
      realPath: realpathSync.native(path),
      id: path,
    }))
    .sort(
      (a, b) =>
        compareCodePoints(a.realPath, b.realPath) ||
        compareCodePoints(a.path, b.path),
    )
}

async function fixture(
  run: (
    root: string,
    file: (path: string, code?: string) => string,
  ) => Promise<void>,
) {
  const root = realpathSync.native(
    mkdtempSync(join(tmpdir(), 'devup-p2-cycles-')),
  )
  const file = (path: string, code = 'export {}') => {
    const target = join(root, path)
    mkdirSync(dirname(target), { recursive: true })
    writeFileSync(target, code)
    return target
  }
  try {
    await run(root, file)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}

it.each([false, true])(
  'retains a finite growing backedge with reversed seeds=%s',
  async (reverse) => {
    await fixture(async (root, file) => {
      // Given a real directory self-link and an import growing its lexical spelling.
      const source = file('physical/main.ts', "import './again/main.ts'")
      const plain = file('plain.ts')
      symlinkSync(dirname(source), join(root, 'physical/again'), 'junction')
      const files = enumerateProductionSourceFiles({
        roots: [],
        sourceFiles: [source, plain],
      })
      // When file recursion meets the same active physical source.
      const result = await collectProductionFileManifest({
        contexts: [
          {
            key: 'cycle',
            files: reverse ? files.toReversed() : files,
            resolverOptions: { cwd: root },
            toId: (path) => path,
          },
        ],
      })
      // Then the selected physical backedge is reserved once without infinite lexical expansion.
      const back = join(root, 'physical/again/main.ts')
      expect(result).toEqual(memberships([source, back, plain], 'cycle'))
    })
  },
)

it('keeps both nearest package copies when physical importers have two lexical aliases', async () => {
  await fixture(async (root, file) => {
    // Given two independent aliases to one importer, with distinct ancestor installations.
    const physical = file('physical/main.ts', "import 'palette'")
    const targets = ['left', 'right'].map((side) => {
      file(`${side}/node_modules/palette/package.json`, '{"main":"value.js"}')
      const target = file(`${side}/node_modules/palette/value.js`)
      symlinkSync(dirname(physical), join(root, `${side}/alias`), 'junction')
      return target
    })
    const files = enumerateProductionSourceFiles({
      roots: [],
      sourceFiles: ['left/alias/main.ts', 'right/alias/main.ts'],
      cwd: root,
    })
    const context = {
      key: 'aliases',
      files,
      resolverOptions: { cwd: root },
      toId: (path: string) => path,
    }
    // When the two seed orders independently close their importer-sensitive dependencies.
    const first = await collectProductionFileManifest({ contexts: [context] })
    const second = await collectProductionFileManifest({
      contexts: [{ ...context, files: files.toReversed() }],
    })
    // Then neither global physical visitation nor arrival order can discard a target.
    expect(second).toEqual(first)
    expect(first).toEqual(
      memberships([...files.map(({ path }) => path), ...targets], 'aliases'),
    )
  })
})

it('replays local adjacency when an earlier incoming ancestry cut a later alias', async () => {
  await fixture(async (root, file) => {
    // Given A -> B -> alias(A); reaching B first must expand alias(A)'s nearest package.
    const a = file('physical/a.ts', "import 'palette'; import '../bridge/b.ts'")
    const b = file('bridge/b.ts', "import '../alternate/alias/a.ts'")
    file('alternate/node_modules/palette/package.json', '{"main":"value.js"}')
    const leaf = file('alternate/node_modules/palette/value.js')
    symlinkSync(dirname(a), join(root, 'alternate/alias'), 'junction')
    const files = enumerateProductionSourceFiles({
      roots: [],
      sourceFiles: [a, b],
    })
    const context = {
      key: 'replay',
      files,
      resolverOptions: { cwd: root },
      toId: (path: string) => path,
    }
    // When independent incoming paths revisit a lexical node after a physical backedge cut.
    const first = await collectProductionFileManifest({ contexts: [context] })
    const second = await collectProductionFileManifest({
      contexts: [{ ...context, files: files.toReversed() }],
    })
    // Then ancestry-pruned subtree memoization cannot hide the alternate lexical dependency.
    expect(second).toEqual(first)
    expect(first).toEqual(
      memberships([a, b, leaf, join(root, 'alternate/alias/a.ts')], 'replay'),
    )
  })
})

it.each([false, true])(
  'cuts package-link recursion with reversed seeds=%s',
  async (reverse) => {
    await fixture(async (root, file) => {
      // Given a physically recursive admitted package installation.
      const entry = file('src/main.ts')
      const plain = file('src/plain.ts')
      file('node_modules/@devup-ui/ui/package.json', '{}')
      const source = file('node_modules/@devup-ui/ui/view.ts')
      const scope = join(
        root,
        'node_modules/@devup-ui/ui/node_modules/@devup-ui',
      )
      mkdirSync(scope, { recursive: true })
      symlinkSync(dirname(source), join(scope, 'ui'), 'junction')
      // When distribution jobs inherit active physical-package ancestry.
      const result = await collectProductionFileManifest({
        contexts: [
          {
            key: 'packages',
            files: [
              ...enumerateProductionSourceFiles({ roots: ['src'], cwd: root }),
            ].sort((a, b) =>
              reverse
                ? b.path.localeCompare(a.path)
                : a.path.localeCompare(b.path),
            ),
            resolverOptions: { cwd: root },
            toId: (path) => path,
          },
        ],
      })
      // Then the original finite package inventory is enough; no recursive synthetic aliases exist.
      expect(result).toEqual(memberships([entry, source, plain], 'packages'))
    })
  },
)

it.each([3, 6, 9])(
  'closes an external diamond ladder with %s levels under a bounded witness',
  async (levels) => {
    await fixture(async (root, file) => {
      // Given an external DAG with convergent paths and incomparable file ancestries.
      const paths = [
        file(
          'src/main.ts',
          "import '../outside/0-left.js'; import '../outside/0-right.js'",
        ),
      ]
      for (let level = 0; level < levels; level++) {
        const code =
          level + 1 === levels
            ? "import './leaf.js'"
            : `import './${level + 1}-left.js'; import './${level + 1}-right.js'`
        for (const side of ['left', 'right'])
          paths.push(file(`outside/${level}-${side}.js`, code))
      }
      paths.push(file('outside/leaf.js'))
      let mappings = 0
      const context = {
        key: 'diamond',
        files: enumerateProductionSourceFiles({ roots: ['src'], cwd: root }),
        resolverOptions: { cwd: root },
        toId: (path: string) => {
          if (++mappings > 8000)
            throw new RangeError('bounded diamond work exceeded')
          return path
        },
      }
      // When opposite seed orders close each independently retained obligation.
      const first = await collectProductionFileManifest({ contexts: [context] })
      const second = await collectProductionFileManifest({
        contexts: [{ ...context, files: context.files.toReversed() }],
      })
      // Then full physical membership survives; the witness exposes rather than hides replay growth.
      expect(second).toEqual(first)
      expect(first).toEqual(memberships(paths, 'diamond'))
      console.info(
        JSON.stringify({
          diamondLevels: levels,
          records: first.length,
          mappingsBothOrders: mappings,
        }),
      )
    })
  },
)
