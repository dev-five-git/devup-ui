import * as fs from 'node:fs'
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

import { expect, it, spyOn } from 'bun:test'

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
    mkdtempSync(join(tmpdir(), 'devup-p2-packages-')),
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

it('enumerates implicit and exact admitted distributions when none is imported', async () => {
  await fixture(async (root, file) => {
    // Given implicit scopes, exact include, a prefix impostor and own output.
    const entry = file('src/main.ts')
    const kept = ['@devup-ui/ui', '@devup-editor/editor', 'included'].map(
      (name) => {
        file(`node_modules/${name}/package.json`, '{}')
        return file(`node_modules/${name}/dist/view.test.js`)
      },
    )
    file('node_modules/included-other/package.json', '{}')
    file('node_modules/included-other/absent.js')
    const own = file('build/dist/poison.ts', "import 'broken'")
    const files = enumerateProductionSourceFiles({ roots: ['src'], cwd: root })
    // When dormant closure discovers admitted installations, not blanket dependencies.
    const result = await collectProductionFileManifest({
      include: ['included', 'missing'],
      exclude: [dirname(own)],
      contexts: [
        {
          key: 'packages',
          files,
          resolverOptions: { cwd: root },
          toId: (path) => path,
        },
      ],
    })
    // Then installed dist/test source is retained independently of import or needle text.
    expect(result.map(({ path }) => path)).toEqual([entry, ...kept].sort())
  })
})

it('discovers nested installations when a distribution and outside source are reached', async () => {
  await fixture(async (root, file) => {
    // Given a direct included distribution and a non-admitted outside intermediary.
    const entry = file('src/main.ts', "import '../outside/bridge.js'")
    const bridge = file('outside/bridge.js', "import 'ordinary'")
    file('outside/node_modules/ordinary/package.json', '{"main":"index.js"}')
    const ordinary = file('outside/node_modules/ordinary/index.js')
    file('node_modules/included/package.json', '{}')
    const distribution = file(
      'node_modules/included/dist/view.js',
      "import './other.js'",
    )
    const sibling = file('node_modules/included/dist/other.js')
    const nested = [
      'node_modules/included/dist',
      'outside/node_modules/ordinary',
    ].map((base) => {
      file(`${base}/node_modules/@devup-ui/nested/package.json`, '{}')
      return file(`${base}/node_modules/@devup-ui/nested/unused.js`)
    })
    const files = enumerateProductionSourceFiles({ roots: ['src'], cwd: root })
    // When installation discovery repeats at newly reached physical source locations.
    const result = await collectProductionFileManifest({
      include: ['included'],
      contexts: [
        {
          key: 'nested',
          files,
          resolverOptions: { cwd: root },
          toId: (path) => path,
        },
      ],
    })
    // Then both nested physical copies survive with the intermediary closure.
    expect(result.map(({ path }) => path)).toEqual(
      [entry, bridge, ordinary, distribution, sibling, ...nested].sort(),
    )
  })
})

it('preserves package aliases when a workspace distribution is linked', async () => {
  await fixture(async (root, file) => {
    // Given a linked implicit package and an excluded lexical installation.
    const entry = file('src/main.ts')
    file('workspace/package.json', '{}')
    const physical = file('workspace/view.js')
    mkdirSync(join(root, 'node_modules/@devup-ui'), { recursive: true })
    symlinkSync(
      join(root, 'workspace'),
      join(root, 'node_modules/@devup-ui/linked'),
      'junction',
    )
    symlinkSync(
      join(root, 'workspace'),
      join(root, 'node_modules/@devup-ui/blocked'),
      'junction',
    )
    const lexical = join(root, 'node_modules/@devup-ui/linked/view.js')
    // When only one installation spelling is admitted by exclusions.
    const result = await collectProductionFileManifest({
      exclude: [join(root, 'node_modules/@devup-ui/blocked')],
      contexts: [
        {
          key: 'linked',
          files: enumerateProductionSourceFiles({ roots: ['src'], cwd: root }),
          resolverOptions: { cwd: root },
          toId: (path) => path,
        },
      ],
    })
    // Then source extraction identity remains lexical while physical identity follows the link.
    expect(result).toEqual(
      expect.arrayContaining([
        { context: 'linked', path: entry, realPath: entry, id: entry },
        { context: 'linked', path: lexical, realPath: physical, id: lexical },
      ]),
    )
    expect(result).toHaveLength(2)
  })
})

it('discovers admitted files when a context has cwd but no selected project seeds', async () => {
  await fixture(async (root, file) => {
    // Given an empty project and an implicit installation.
    file('node_modules/@devup-ui/ui/package.json', '{}')
    const path = file('node_modules/@devup-ui/ui/view.js')
    // When the actual context cwd provides the installation origin.
    const result = await collectProductionFileManifest({
      contexts: [
        {
          key: 'empty',
          files: [],
          resolverOptions: { cwd: root },
          toId: (source) => source,
        },
      ],
    })
    // Then empty project inventory does not hide directly admitted source.
    expect(result).toEqual([
      { context: 'empty', path, realPath: path, id: path },
    ])
  })
})

it('excludes physical seeds and scopes when lexical links point into blocked output', async () => {
  await fixture(async (root, file) => {
    // Given linked selected seeds and an implicit scope pointing at excluded physical source.
    const physical = file('blocked/view.ts', "import 'poison'")
    const kept = file('src/kept.ts')
    symlinkSync(dirname(physical), join(root, 'src/alias'), 'junction')
    mkdirSync(join(root, 'node_modules'))
    symlinkSync(
      dirname(physical),
      join(root, 'node_modules/@devup-ui'),
      'junction',
    )
    const files = enumerateProductionSourceFiles({ roots: ['src'], cwd: root })
    // When physical exclusions apply before preparation or scope/source reads.
    const result = await collectProductionFileManifest({
      exclude: [dirname(physical)],
      contexts: [
        {
          key: 'excluded',
          files,
          resolverOptions: { cwd: root },
          toId: (path) => path,
        },
      ],
    })
    // Then no excluded linked source can reach its poisoned import.
    expect(result.map(({ path }) => path)).toEqual([kept])
  })
})

it('preserves directory faults when an existing implicit scope cannot be read', async () => {
  await fixture(async (root, file) => {
    // Given a real installed scope and a narrow filesystem permission failure.
    file('src/main.ts')
    file('node_modules/@devup-ui/ui/package.json', '{}')
    const scope = join(root, 'node_modules/@devup-ui')
    const cause = Object.assign(new Error('scope denied'), {
      code: 'EACCES',
      path: scope,
    })
    const original = fs.readdirSync
    const read = spyOn(fs, 'readdirSync').mockImplementation(
      new Proxy(original, {
        apply(target, receiver, args) {
          if (args[0] === scope) throw cause
          return Reflect.apply(target, receiver, args)
        },
      }),
    )
    try {
      // When importer-local distribution discovery reads that existing scope.
      const error: unknown = await collectProductionFileManifest({
        contexts: [
          {
            key: 'scope',
            files: enumerateProductionSourceFiles({
              roots: ['src'],
              cwd: root,
            }),
            resolverOptions: { cwd: root },
            toId: (path) => path,
          },
        ],
      }).catch((failure: unknown) => failure)
      // Then the original IO failure and lexical repair path survive, not an ordinary miss.
      expect(error).toBe(cause)
    } finally {
      read.mockRestore()
    }
  })
})
