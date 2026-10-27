import * as fs from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, relative } from 'node:path'

import { expect, it } from 'bun:test'

import type { ProductionManifestFile } from '../production-file-manifest'
import { enumerateProductionSourceFiles } from '../production-source-files'
import type { ResolutionInputs } from '../resolution-inputs'

function ordered(records: readonly ProductionManifestFile[]) {
  return records.toSorted((a, b) => {
    for (const field of ['realPath', 'path', 'context', 'id'] as const) {
      if (a[field] < b[field]) return -1
      if (a[field] > b[field]) return 1
    }
    return 0
  })
}

async function fixture(
  run: (
    root: string,
    file: (name: string, code?: string) => string,
  ) => Promise<void>,
) {
  const root = fs.realpathSync.native(
    fs.mkdtempSync(join(tmpdir(), 'devup-p2-file-frontier-')),
  )
  const file = (name: string, code = 'export {}') => {
    const path = join(root, name)
    fs.mkdirSync(dirname(path), { recursive: true })
    fs.writeFileSync(path, code)
    return path
  }
  try {
    await run(root, file)
  } finally {
    fs.rmSync(root, { recursive: true, force: true })
  }
}

const cases = [false, true].flatMap((transitive) =>
  [false, true].flatMap((cutFirst) =>
    [false, true].map((reverse) => ({ transitive, cutFirst, reverse })),
  ),
)

it.each(cases)(
  'opens an external file cut when transitive=$transitive cutFirst=$cutFirst reverse=$reverse',
  async ({ transitive, cutFirst, reverse }) => {
    await fixture(async (root, file) => {
      // Given two intermediary origins whose lexical ranks change actual dispatch order.
      const bridge = transitive ? 'c.ts' : 'b.ts'
      const cut = file(
        `src/${cutFirst ? 'a' : 'z'}-cut.ts`,
        "import '../physical/a.ts'",
      )
      const open = file(
        `src/${cutFirst ? 'z' : 'a'}-open.ts`,
        `import '../bridge/${bridge}'`,
      )
      const a = file(
        'physical/a.ts',
        `import 'palette'; import '../bridge/${bridge}'`,
      )
      const b = file('bridge/b.ts', "import '../alternate/alias/a.ts'")
      const c = transitive ? [file('bridge/c.ts', "import './b.ts'")] : []
      const manifest = file(
        'alternate/node_modules/palette/package.json',
        '{"main":"value.js"}',
      )
      const leaf = file('alternate/node_modules/palette/value.js')
      fs.symlinkSync(dirname(a), join(root, 'alternate/alias'), 'junction')
      const alias = join(root, 'alternate/alias/a.ts')
      const paths = [cut, open, a, b, ...c, alias, leaf]
      const expected = ordered(
        paths.map((path) => ({
          path,
          realPath: path === alias ? a : path,
          context: 'frontier',
          id: relative(root, path),
        })),
      )
      const files = enumerateProductionSourceFiles({
        roots: ['src'],
        cwd: root,
      })
      const inputs: ResolutionInputs[] = []
      const visits: string[] = []
      let mappings = 0
      const { collectProductionFileManifest } =
        await import('../production-file-manifest')
      // When a later genuine origin can reopen the alias, including through c.
      const result = await collectProductionFileManifest({
        contexts: [
          {
            key: 'frontier',
            files: reverse ? files.toReversed() : files,
            resolverOptions: {
              cwd: root,
              onResolutionInputs: (input) => inputs.push(input),
            },
            toId: (path) => {
              if (++mappings > 400)
                throw new RangeError('bounded file frontier work')
              return relative(root, path)
            },
            prepareSource: (path) => {
              visits.push(path)
              return undefined
            },
          },
        ],
      })
      // Then every independent lexical record and its real consulted input survive.
      expect(result).toEqual(expected)
      expect(new Set(visits)).toEqual(new Set(paths))
      expect(inputs.flatMap((input) => input.fileDependencies)).toEqual(
        expect.arrayContaining([manifest, alias, leaf]),
      )
      console.info(
        JSON.stringify({
          event: 'file-frontier',
          transitive,
          cutFirst,
          reverse,
          expected,
          mappings,
          visits,
        }),
      )
    })
  },
)

it.each([false, true])(
  'separates equal context keys when context order reverses=%s',
  async (reverse) => {
    await fixture(async (root, file) => {
      // Given equal keys but different prepared code, aliases, conditions and ID values.
      const entry = file('src/main.ts', "import 'phantom'")
      const environments = ['blue', 'red'].map((color) => {
        const observed: ResolutionInputs[] = []
        const visited: string[] = []
        return {
          color,
          provider: file(`outside/${color}.js`),
          leaf: file(`node_modules/palette/${color}.js`),
          observed,
          visited,
        }
      })
      const manifest = file(
        'node_modules/palette/package.json',
        '{"exports":{"blue":"./blue.js","red":"./red.js"}}',
      )
      const files = enumerateProductionSourceFiles({
        roots: ['src'],
        cwd: root,
      })
      const contexts = environments.map((env) => ({
        key: 'equal',
        files,
        resolverOptions: {
          cwd: root,
          conditions: [env.color],
          alias: { provider: env.provider },
          onResolutionInputs: (input: ResolutionInputs) =>
            env.observed.push(input),
        },
        toId: (path: string) => `${env.color}:${relative(root, path)}`,
        prepareSource: (path: string) => {
          env.visited.push(path)
          return path === entry
            ? `import 'provider'; import 'palette'; export const color='${env.color}'`
            : undefined
        },
      }))
      const expected = ordered(
        environments.flatMap((env) =>
          [entry, env.provider, env.leaf].map((path) => ({
            path,
            realPath: path,
            context: 'equal',
            id: `${env.color}:${relative(root, path)}`,
          })),
        ),
      )
      const { collectProductionFileManifest } =
        await import('../production-file-manifest')
      // When independent context instances converge on the same lexical entry.
      const result = await collectProductionFileManifest({
        contexts: reverse ? contexts.toReversed() : contexts,
      })
      // Then both environments remain in the full union and lexical source provenance.
      expect(result).toEqual(expected)
      for (const env of environments) {
        const paths = [entry, env.provider, env.leaf]
        expect(new Set(env.visited)).toEqual(new Set(paths))
        expect(env.observed.flatMap((input) => input.fileDependencies)).toEqual(
          expect.arrayContaining([entry, manifest, ...paths]),
        )
      }
    })
  },
)
