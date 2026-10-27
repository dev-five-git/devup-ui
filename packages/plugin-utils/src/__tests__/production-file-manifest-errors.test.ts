import * as fs from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import { expect, it, spyOn } from 'bun:test'

import type { ProductionFileManifestOptions } from '../production-file-manifest'
import { enumerateProductionSourceFiles } from '../production-source-files'
import type { ModuleAliasOptions } from '../types'

function sourceContext(root: string) {
  return {
    key: 'errors',
    files: enumerateProductionSourceFiles({ roots: ['src'], cwd: root }),
    resolverOptions: { cwd: root },
    toId: (path: string) => path,
  }
}

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
  const root = fs.realpathSync.native(
    fs.mkdtempSync(join(tmpdir(), 'devup-p2-errors-')),
  )
  const file = (path: string, code = 'export {}') => {
    const target = join(root, path)
    fs.mkdirSync(dirname(target), { recursive: true })
    fs.writeFileSync(target, code)
    return target
  }
  try {
    await run(root, file)
  } finally {
    fs.rmSync(root, { recursive: true, force: true })
  }
}

it('keeps ignores exclusions and misses pathless when real source resolves alongside them', async () => {
  await fixture(async (root, file) => {
    // Given ignored, excluded, unresolved and ordinary rewritten requests.
    const entry = file(
      'src/main.ts',
      "import 'ignored'; import 'blocked'; import 'external'; import './missing'; import 'chosen';",
    )
    const chosen = file('outside/chosen.js')
    const blocked = file('blocked/value.js', "import 'poison'")
    const files = enumerateProductionSourceFiles({ roots: ['src'], cwd: root })
    // When the same resolver returns all four distinct outcome classes.
    const result = await collectProductionFileManifest({
      exclude: [dirname(blocked)],
      contexts: [
        {
          key: 'outcomes',
          files,
          resolverOptions: {
            cwd: root,
            alias: [
              {
                name: 'ignored',
                alias: [join(root, 'absent'), false],
                onlyModule: true,
              },
              { name: 'blocked', alias: blocked },
              { name: 'chosen', alias: [join(root, 'missing'), chosen] },
            ],
          },
          toId: (path) => path,
        },
      ],
    })
    // Then only actual physical resolutions receive memberships.
    expect(result.map(({ path }) => path)).toEqual([entry, chosen].sort())
  })
})

it.each(['cycle', 'manifest', 'exports', 'candidate', 'config'])(
  'preserves fatal %s failures when closure cannot resolve an edge',
  async (kind) => {
    await fixture(async (root, file) => {
      // Given a real fatal resolver boundary with a tempting later alias candidate.
      const entry = file('src/main.ts', "import 'provider'")
      const later = file('later.js')
      const alias: ModuleAliasOptions =
        kind === 'cycle'
          ? { provider: ['nested', later], nested: 'provider' }
          : kind === 'candidate'
            ? { provider: join(root, 'absent') }
            : { provider: ['broken', later] }
      const manifest = file(
        'node_modules/broken/package.json',
        kind === 'manifest' ? '{' : '{"exports":{"browser":"./absent.js"}}',
      )
      const config = kind === 'config' ? file('tsconfig.json', '{') : undefined
      // When existing resolver setup/edge failure is encountered.
      const result = collectProductionFileManifest({
        contexts: [
          {
            key: 'fatal',
            files: enumerateProductionSourceFiles({
              roots: ['src'],
              cwd: root,
            }),
            resolverOptions: { cwd: root, alias, conditions: ['browser'] },
            toId: (path) => path,
          },
        ],
      })
      // Then the located existing failure is never converted into a miss/fallback.
      await expect(result).rejects.toThrow(
        kind === 'config' ? config : kind === 'manifest' ? manifest : entry,
      )
    })
  },
)

it.each(['mdx', 'mdown'])(
  'rejects missing preparation when Markdown extension is %s',
  async (extension) => {
    await fixture(async (root, file) => {
      // Given selected physical Markdown that cannot be interpreted as compiled JavaScript.
      const entry = file(`src/page.${extension}`, '# Raw source')
      // When no preparation is available before dormant closure.
      const result = collectProductionFileManifest({
        contexts: [
          {
            key: 'unprepared',
            files: enumerateProductionSourceFiles({
              roots: ['src'],
              cwd: root,
              includeMdx: [`.${extension}`],
            }),
            resolverOptions: { cwd: root, includeMdx: [`.${extension}`] },
            toId: (path) => path,
          },
        ],
      })
      // Then the error locates the physical file and asks for preparation, not a virtual workaround.
      await expect(result).rejects.toThrow(`${entry}:1:1:`)
    })
  },
)

it.each([false, true])(
  'retains compiler cause when preparer rejects asynchronously=%s',
  async (asyncHook) => {
    await fixture(async (root, file) => {
      // Given the original compiler failure and real prepared-source filename.
      const entry = file('src/page.ts')
      const cause = Object.assign(new Error('compiler failed'), {
        line: 3,
        column: 4,
      })
      const fail = () => {
        throw cause
      }
      // When caller preparation fails at its actual boundary.
      const error: unknown = await collectProductionFileManifest({
        contexts: [
          {
            ...sourceContext(root),
            prepareSource: asyncHook ? async () => fail() : fail,
          },
        ],
      }).catch((failure: unknown) => failure)
      // Then location is honestly compiled output and cause identity survives.
      expect(error).toMatchObject({ cause })
      if (!(error instanceof Error))
        throw new TypeError('Expected located compiler failure')
      expect(error.message).toContain(`${entry}:3:4`)
      expect(error.message).toContain('in compiled output')
    })
  },
)

it('preserves type getter cause when a prepared mode cannot be read', async () => {
  await fixture(async (root, file) => {
    // Given a typed prepared boundary with a throwing mode getter.
    const entry = file('src/page.ts')
    const cause = new TypeError('mode getter')
    const prepared = {
      code: '',
      get sourceType(): 'compiled-mdx' {
        throw cause
      },
    }
    // When prepared-source parsing reads the hook value.
    const error: unknown = await collectProductionFileManifest({
      contexts: [{ ...sourceContext(root), prepareSource: () => prepared }],
    }).catch((failure: unknown) => failure)
    // Then both the boundary and original getter remain in the cause chain.
    expect(error).toMatchObject({ cause: { cause } })
    if (!(error instanceof Error)) throw new TypeError('Expected mode failure')
    expect(error.message).toContain(`${entry}:1:1:`)
  })
})

it('observes lexical config probes and source reads when inherited paths resolve outside roots', async () => {
  await fixture(async (root, file) => {
    // Given inherited config aliases and an earlier nonexistent candidate.
    const config = file(
      'tsconfig.json',
      '{"extends":"./base","compilerOptions":{}}',
    )
    const base = file(
      'base.json',
      '{"compilerOptions":{"paths":{"value":["absent","outside/value.ts"]}}}',
    )
    const entry = file('src/main.ts', "import 'value'")
    const target = file('outside/value.ts')
    const observed: {
      readonly fileDependencies: readonly string[]
      readonly missingDependencies: readonly string[]
    }[] = []
    // When actual resolver/setup and source reads report their existing provenance.
    await collectProductionFileManifest({
      contexts: [
        {
          key: 'inputs',
          files: enumerateProductionSourceFiles({ roots: ['src'], cwd: root }),
          resolverOptions: {
            cwd: root,
            onResolutionInputs: (inputs) => observed.push(inputs),
          },
          toId: (path) => path,
        },
      ],
    })
    // Then lexical consulted inputs and misses remain repairable.
    expect(
      observed.flatMap(({ fileDependencies }) => fileDependencies),
    ).toEqual(expect.arrayContaining([config, base, entry, target]))
    expect(
      observed.flatMap(({ missingDependencies }) => missingDependencies),
    ).toContain(join(root, 'absent'))
  })
})

it('retains read cause when an ordinary physical source becomes unreadable', async () => {
  await fixture(async (root, file) => {
    // Given a real physical source and a narrow deterministic OS permission failure.
    const entry = file('src/main.ts')
    const cause = Object.assign(new Error('read denied'), { code: 'EACCES' })
    const original = fs.readFileSync
    const read = spyOn(fs, 'readFileSync').mockImplementation(
      new Proxy(original, {
        apply(target, receiver, args) {
          if (args[0] === entry) throw cause
          return Reflect.apply(target, receiver, args)
        },
      }),
    )
    try {
      // When actual raw source reading fails after enumeration.
      const error: unknown = await collectProductionFileManifest({
        contexts: [sourceContext(root)],
      }).catch((failure: unknown) => failure)
      // Then closure fails with the actual source location and original IO cause.
      expect(error).toMatchObject({ cause })
      if (!(error instanceof Error))
        throw new TypeError('Expected read failure')
      expect(error.message).toContain(entry)
    } finally {
      read.mockRestore()
    }
  })
})
