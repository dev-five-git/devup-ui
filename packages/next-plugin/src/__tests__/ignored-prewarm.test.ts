import { join } from 'node:path'

import type { StaticImportGraph } from '@devup-ui/plugin-utils'
import { expect, it } from 'bun:test'

import { collectPrewarmFiles } from '../prewarm'
import { runPrewarm } from '../prewarm-run'
import { createAppContext } from '../session'
import { createWasm, withModuleResolver } from '../wasm'
import { installProjectHooks, makeProject } from './project'

installProjectHooks()

it.each(['package', 'relative', 'esm-sibling'] as const)(
  'omits ignored %s outcomes before reading or prewarming a resource',
  (kind) => {
    // Given a real included package, poisoned ignored bytes and a preparation observer.
    const root = makeProject({
      'src/page.tsx': '',
      'node_modules/@acme/ui/package.json': JSON.stringify({
        name: '@acme/ui',
        exports: './index.cjs',
      }),
      'node_modules/@acme/ui/index.cjs': 'import "./ignored.ts"',
      'node_modules/@acme/ui/index.mjs': 'import "./ignored.ts"',
      'node_modules/@acme/ui/ignored.ts': 'import "./unresolvable.ts"',
    })
    const page = join(root, 'src/page.tsx')
    const graph: StaticImportGraph = {
      files: [page],
      fileSet: new Set([page]),
      staticImports: new Map(),
      staticImporters: new Map(),
      dynamicImports: new Map(),
      dynamicTargets: new Set(),
      externalImports: new Map([[page, new Set(['@acme/ui'])]]),
    }
    const ignoredRequest = {
      package: '@acme/ui',
      relative: './ignored.ts',
      'esm-sibling': join(root, 'node_modules/@acme/ui/index.mjs'),
    }[kind]
    const prepared: string[] = []
    process.chdir(root)
    // When collection uses the shared false-alias resolver.
    const files = collectPrewarmFiles({
      root,
      graph,
      expectedBaseFiles: ['src/page.tsx'],
      libPackage: '@devup-ui/react',
      include: ['@acme/ui'],
      prewarmAll: false,
      resolver: {
        alias: { [ignoredRequest]: false },
        prepareSource: (filename) => {
          prepared.push(filename)
          return undefined
        },
      },
    })
    // Then no ignored path enters the extraction/numbering set or source preparation.
    const expectedFiles = {
      package: ['src/page.tsx'],
      relative: ['node_modules/@acme/ui/index.mjs', 'src/page.tsx'],
      'esm-sibling': ['src/page.tsx'],
    }[kind]
    expect(files).toEqual(expectedFiles)
    expect(prepared).toEqual(
      {
        package: [],
        relative: [
          join(root, 'node_modules/@acme/ui/index.cjs'),
          join(root, 'node_modules/@acme/ui/index.mjs'),
        ],
        'esm-sibling': [join(root, 'node_modules/@acme/ui/index.cjs')],
      }[kind],
    )
    const engine = withModuleResolver(createWasm(root), root, {
      alias: { [ignoredRequest]: false },
    })
    engine.seedFileMap(files)
    const output = runPrewarm({
      context: createAppContext({}, {}),
      engine,
      files,
      collectMs: undefined,
    })
    expect(output.files).toEqual(expectedFiles)
    expect(Object.keys(JSON.parse(engine.exportFileMap())).sort()).toEqual(
      expectedFiles,
    )
  },
)
