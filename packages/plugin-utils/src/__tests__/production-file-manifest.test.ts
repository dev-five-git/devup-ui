import * as fs from 'node:fs'
import {
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, relative } from 'node:path'

import { expect, it, spyOn } from 'bun:test'

import { collectNumberedFiles } from '../numbering'
import type { ProductionFileManifestOptions } from '../production-file-manifest'
import { enumerateProductionSourceFiles } from '../production-source-files'

async function collectProductionFileManifest(
  options: ProductionFileManifestOptions,
) {
  return (
    await import('../production-file-manifest')
  ).collectProductionFileManifest(options)
}

async function fixture(
  run: (
    root: string,
    file: (path: string, code?: string) => string,
  ) => Promise<void>,
) {
  const root = realpathSync.native(
    mkdtempSync(join(tmpdir(), 'devup-p2-union-')),
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

it('unions independent conditional file results when contexts and seeds reverse', async () => {
  await fixture(async (root, file) => {
    // Given different styled FILE exports, an outside intermediary and two ID conventions.
    const entry = file(
      'src/main.ts',
      "import 'palette'; import '../outside/bridge';",
    )
    const plain = file('src/plain.spec.ts')
    const bridge = file('outside/bridge.js', "export * from 'plain';")
    file('node_modules/plain/package.json', '{"main":"leaf.js"}')
    const leaf = file('node_modules/plain/leaf.js')
    file(
      'node_modules/palette/package.json',
      '{"exports":{"browser":"./browser.js","react-server":"./server.js"}}',
    )
    const browser = file(
      'node_modules/palette/browser.js',
      "import '@devup-ui/react';",
    )
    const server = file(
      'node_modules/palette/server.js',
      "import '@devup-ui/react';",
    )
    const files = enumerateProductionSourceFiles({ roots: ['src'], cwd: root })
    const contexts = ['browser', 'react-server'].map((key) => ({
      key,
      files,
      resolverOptions: { cwd: root, conditions: [key] },
      toId: (path: string) => `${key}:${relative(root, path)}`,
    }))
    // When the same physical inputs arrive in opposite orders.
    const first = await collectProductionFileManifest({ contexts })
    const reversed = await collectProductionFileManifest({
      contexts: contexts
        .toReversed()
        .map((context) => ({ ...context, files: files.toReversed() })),
    })
    // Then membership is the independent result union, never a flattened condition winner.
    expect(reversed).toEqual(first)
    const expected = contexts.flatMap(({ key }) =>
      [entry, plain, bridge, leaf, key === 'browser' ? browser : server].map(
        (path) => ({
          context: key,
          path,
          realPath: path,
          id: `${key}:${relative(root, path)}`,
        }),
      ),
    )
    expect(first).toEqual(expect.arrayContaining(expected))
    expect(first).toHaveLength(expected.length)
    expect(Object.isFrozen(first) && first.every(Object.isFrozen)).toBe(true)
    expect(
      collectNumberedFiles({
        roots: [join(root, 'src')],
        needles: ['@devup-ui/react'],
      }),
    ).toEqual([])
  })
})

it.each([[undefined], [false], [true], [[]], [['.md', '.mdown']]])(
  'retains exact source selection when selection is %j',
  async (includeMdx) => {
    await fixture(async (root, file) => {
      // Given eight source extensions and distinct Markdown sources without styling needles.
      const js = 'ts tsx mts cts js jsx mjs cjs'
        .split(' ')
        .map((extension) => file(`src/source.${extension}`))
      const md = file('src/page.md')
      const mdx = file('src/page.mdx')
      const custom = file('src/page.mdown')
      const selection = includeMdx === undefined ? {} : { includeMdx }
      const files = enumerateProductionSourceFiles({
        roots: ['src'],
        cwd: root,
        ...selection,
      })
      // When physical selection is carried unchanged into dormant closure.
      const result = await collectProductionFileManifest({
        contexts: [
          {
            key: 'selection',
            files,
            resolverOptions: { cwd: root, ...selection },
            toId: (path) => path,
            prepareSource: () => '',
          },
        ],
      })
      // Then only the exact selected physical records survive.
      const extra =
        includeMdx === true
          ? [mdx]
          : Array.isArray(includeMdx) && includeMdx.length
            ? [md, custom]
            : []
      expect(result.map(({ path }) => path)).toEqual([...js, ...extra].sort())
    })
  },
)

it('deduplicates exact memberships when overlapping P1 seeds repeat', async () => {
  await fixture(async (root, file) => {
    // Given repeated lexical seeds and a selected physical source.
    const path = file('src/one.ts')
    const files = enumerateProductionSourceFiles({ roots: ['src'], cwd: root })
    // When the same context repeats seed records.
    const result = await collectProductionFileManifest({
      contexts: [
        {
          key: 'one',
          files: [...files, ...files],
          resolverOptions: { cwd: root },
          toId: () => 'original-id',
        },
      ],
    })
    // Then one exact extraction membership remains.
    expect(result).toEqual([
      { context: 'one', path, realPath: path, id: 'original-id' },
    ])
  })
})

it.each([4, 8])(
  'bounds real distribution work with %s implicit packages',
  async (count) => {
    await fixture(async (root, file) => {
      // Given independent import-free distributions, including test-named source.
      const paths = [file('src/main.ts')]
      for (let index = 0; index < count; index++) {
        const base = `node_modules/@devup-ui/p${index}`
        file(`${base}/package.json`, '{}')
        for (const name of ['a.js', 'b.test.js', 'c.spec.js'])
          paths.push(file(`${base}/dist/${name}`))
      }
      const files = enumerateProductionSourceFiles({
        roots: ['src'],
        cwd: root,
      })
      let mappings = 0
      let filesystem = 0
      const meter = <T extends (...args: never[]) => unknown>(
        operation: T,
      ): T =>
        new Proxy(operation, {
          apply(target, receiver, args) {
            if (++filesystem > count * 4000)
              throw new RangeError(
                'distribution filesystem work budget exceeded',
              )
            return Reflect.apply(target, receiver, args)
          },
        })
      const originalDirectory = fs.readdirSync
      const originalProbe = fs.statSync
      const directories = spyOn(fs, 'readdirSync').mockImplementation(
        meter(originalDirectory),
      )
      const probes = spyOn(fs, 'statSync').mockImplementation(
        meter(originalProbe),
      )
      try {
        // When real discovery, P1 enumeration and ID mapping run under generous work budgets.
        const result = await collectProductionFileManifest({
          contexts: [
            {
              key: 'budget',
              files,
              resolverOptions: { cwd: root },
              toId: (path) => {
                if (++mappings > count * 250)
                  throw new RangeError(
                    'distribution traversal work budget exceeded',
                  )
                return path
              },
            },
          ],
        })
        // Then every independently expected record survives without permutation replay.
        expect(result).toEqual(
          paths.sort().map((path) => ({
            context: 'budget',
            path,
            realPath: path,
            id: path,
          })),
        )
        expect(mappings).toBeLessThanOrEqual(count * 250)
        expect(filesystem).toBeLessThanOrEqual(count * 4000)
        console.info(
          JSON.stringify({
            distributionPackages: count,
            records: result.length,
            mappings,
            filesystem,
          }),
        )
      } finally {
        directories.mockRestore()
        probes.mockRestore()
      }
    })
  },
)
