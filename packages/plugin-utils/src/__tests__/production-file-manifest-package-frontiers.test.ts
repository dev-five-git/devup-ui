import * as fs from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { expect, it } from 'bun:test'

import { enumerateProductionSourceFiles } from '../production-source-files'
import type { ResolutionInputs } from '../resolution-inputs'

function records(paths: readonly string[]) {
  return paths
    .map((path) => ({
      path,
      realPath: fs.realpathSync.native(path),
      context: 'packages',
      id: path,
    }))
    .sort((a, b) => {
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
    fs.mkdtempSync(join(tmpdir(), 'devup-p2-package-frontier-')),
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

const cases = [false, true].flatMap((cutFirst) =>
  [false, true].map((reverse) => ({ cutFirst, reverse })),
)

it.each(cases)(
  'opens a linked inventory when cutFirst=$cutFirst reverse=$reverse',
  async ({ cutFirst, reverse }) => {
    await fixture(async (root, file) => {
      // Given an inventory-derived bridge and a later intermediary origin without PACKAGE p.
      const cut = file(`src/${cutFirst ? 'a' : 'z'}-cut.ts`)
      const open = file(
        `src/${cutFirst ? 'z' : 'a'}-open.ts`,
        "import '../outside/b.ts'",
      )
      const b = file('outside/b.ts')
      const manifest = file('node_modules/@devup-ui/p/package.json', '{}')
      const source = file('node_modules/@devup-ui/p/view.js', "import 'bridge'")
      const scope = join(root, 'outside/node_modules/@devup-ui')
      fs.mkdirSync(scope, { recursive: true })
      const linked = join(scope, 'alias-p')
      fs.symlinkSync(dirname(source), linked, 'junction')
      const alias = join(linked, 'view.js')
      const expected = records([cut, open, b, source, alias])
      const files = enumerateProductionSourceFiles({
        roots: ['src'],
        cwd: root,
      })
      const inputs: ResolutionInputs[] = []
      const visits: string[] = []
      let mappings = 0
      const { collectProductionFileManifest } =
        await import('../production-file-manifest')
      // When the later genuine FILE path reopens the physically cut package spelling.
      const result = await collectProductionFileManifest({
        contexts: [
          {
            key: 'packages',
            files: reverse ? files.toReversed() : files,
            resolverOptions: {
              cwd: root,
              alias: { bridge: b },
              onResolutionInputs: (input) => inputs.push(input),
            },
            toId: (path) => {
              if (++mappings > 500)
                throw new RangeError('bounded package frontier work')
              return path
            },
            prepareSource: (path) => {
              visits.push(path)
              return undefined
            },
          },
        ],
      })
      // Then the complete finite inventory includes the newly permitted lexical source.
      expect(result).toEqual(expected)
      expect(new Set(visits)).toEqual(new Set([cut, open, b, source, alias]))
      expect(inputs.flatMap((input) => input.fileDependencies)).toEqual(
        expect.arrayContaining([manifest, join(linked, 'package.json'), alias]),
      )
      console.info(
        JSON.stringify({
          event: 'package-frontier',
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
  'keeps lexical nearest package leaves when linked origins reverse=%s',
  async (reverse) => {
    await fixture(async (root, file) => {
      // Given two lexical installations of one physical importer with distinct nearest palettes.
      file('workspace/package.json', '{}')
      const physical = file('workspace/view.js', "import 'palette'")
      const declarations = ['left', 'right'].map((side) => {
        const seed = file(`src/${side}.ts`, `import '../${side}/bridge.ts'`)
        const bridge = file(`${side}/bridge.ts`)
        const manifest = file(
          `${side}/node_modules/palette/package.json`,
          '{"main":"value.js"}',
        )
        const leaf = file(`${side}/node_modules/palette/value.js`)
        const scope = join(root, side, 'node_modules/@devup-ui')
        fs.mkdirSync(scope, { recursive: true })
        const linked = join(scope, 'shared')
        fs.symlinkSync(dirname(physical), linked, 'junction')
        return {
          seed,
          bridge,
          leaf,
          manifest,
          linked,
          source: join(linked, 'view.js'),
        }
      })
      const expected = records(
        declarations.flatMap(({ seed, bridge, source, leaf }) => [
          seed,
          bridge,
          source,
          leaf,
        ]),
      )
      const files = enumerateProductionSourceFiles({
        roots: ['src'],
        cwd: root,
      })
      const inputs: ResolutionInputs[] = []
      let mappings = 0
      const { collectProductionFileManifest } =
        await import('../production-file-manifest')
      // When importer-local discovery expands both genuine lexical installations.
      const result = await collectProductionFileManifest({
        contexts: [
          {
            key: 'packages',
            files: reverse ? files.toReversed() : files,
            resolverOptions: {
              cwd: root,
              onResolutionInputs: (input) => inputs.push(input),
            },
            toId: (path) => {
              if (++mappings > 500)
                throw new RangeError('bounded lexical package work')
              return path
            },
          },
        ],
      })
      // Then no global physical winner can lose a lexical record, leaf or consulted manifest.
      expect(result).toEqual(expected)
      for (const declaration of declarations) {
        expect(inputs.flatMap((input) => input.fileDependencies)).toEqual(
          expect.arrayContaining([
            declaration.manifest,
            join(declaration.linked, 'package.json'),
            declaration.source,
            declaration.leaf,
          ]),
        )
        expect(inputs.flatMap((input) => input.missingDependencies)).toContain(
          join(declaration.linked, 'node_modules/palette/package.json'),
        )
      }
    })
  },
)
