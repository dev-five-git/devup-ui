import * as fs from 'node:fs'
import { join } from 'node:path'

import { expect, it, spyOn } from 'bun:test'

import {
  __setOxcParserForTest,
  buildPreparationImportGraph,
  createModulePathResolver,
} from '../import-graph'
import { PreparationInputs } from '../preparation-inputs'
import { collectProductionFileManifest } from '../production-file-manifest'
import type { ResolutionInputs } from '../resolution-inputs'
import { createPreparedFixture } from './prepared-graph-fixture'

let root: string
const file = createPreparedFixture((path) => {
  root = path
})

it('reuses selecting texts without sharing condition resolution or losing cache-hit observations', () => {
  // Given inherited packaged config and two condition branches with a missing earlier alias.
  const config = file(
    'tsconfig.json',
    '{"extends":"preset","compilerOptions":{"paths":{"local":["absent","red.js"]}}}',
  )
  const scope = file('package.json', '{"name":"fixture"}')
  const preset = file(
    'node_modules/preset/package.json',
    '{"tsconfig":"base.json"}',
  )
  const base = file('node_modules/preset/base.json', '{}')
  const manifest = file(
    'node_modules/palette/package.json',
    '{"exports":{"browser":"./browser.js","node":"./node.js"}}',
  )
  const browser = file('node_modules/palette/browser.js')
  const server = file('node_modules/palette/node.js')
  const red = file('red.js')
  const importer = file('main.js')
  const epoch = new PreparationInputs()
  const observations: ResolutionInputs[] = []
  const read = spyOn(fs, 'readFileSync')
  try {
    // When distinct resolvers and repeated requests share only raw input facts.
    for (const [condition, path] of [
      ['browser', browser],
      ['node', server],
    ] as const) {
      const resolver = createModulePathResolver(
        {
          cwd: root,
          conditions: [condition],
          onResolutionInputs: (inputs) => observations.push(inputs),
        },
        undefined,
        epoch.collect,
      )
      for (let repeat = 0; repeat < 2; repeat++) {
        expect(resolver('palette', importer)).toMatchObject({ path })
        expect(observations.at(-1)).toEqual({
          fileDependencies: [
            config,
            scope,
            preset,
            base,
            manifest,
            path,
          ].sort(),
          missingDependencies: [join(root, 'node_modules/preset.json')],
        })
        expect(resolver('local', importer)).toMatchObject({ path: red })
        expect(observations.at(-1)?.missingDependencies).toContain(
          join(root, 'absent'),
        )
      }
    }
    // Then each selecting text was actually read once, but every policy resolution was observed.
    for (const path of [config, scope, preset, base, manifest])
      expect(read.mock.calls.filter((args) => args[0] === path)).toHaveLength(1)
    expect(observations).toHaveLength(10)
    fs.writeFileSync(manifest, '{"exports":"./node.js"}')
    expect(
      createModulePathResolver({ cwd: root, conditions: ['browser'] })(
        'palette',
        importer,
      ),
    ).toMatchObject({ path: server })
  } finally {
    read.mockRestore()
  }
})

it('retains independent prepared identities, asynchronous calls and grammar while sharing scanner facts', async () => {
  // Given one physical TS source and distinct outputs from independent preparers.
  const entry = file('src/main.ts', 'raw source is not prepared output')
  const a = file('outside/a.js')
  const b = file('outside/b.js')
  const epoch = new PreparationInputs()
  const visits: string[] = []
  const contexts = ['ts', 'jsx'].map((key) => ({
    key,
    files: [{ path: entry, realPath: entry }],
    resolverOptions: { cwd: root },
    toId: (path: string) => `${key}:${path}`,
    prepareSource: async (path: string) => {
      visits.push(`${key}:${path}`)
      return path !== entry
        ? undefined
        : key === 'ts'
          ? { code: "const x=a < b; import('../outside/a.js');" }
          : {
              code: "const x=<div>import('../outside/a.js'){import('../outside/b.js')}</div>;",
              sourceType: 'compiled-mdx' as const,
              map: { version: 3, mappings: '' },
            }
    },
  }))
  // When collector contexts share the epoch, not their prepared outputs or adjacency.
  const result = await collectProductionFileManifest(
    { contexts, entryClosure: true },
    epoch,
  )
  // Then the ordinary TS and compiled JSX edges remain independent and callbacks keep their IDs.
  expect(
    result.map(({ context, path, id }) => [context, path, id]).sort(),
  ).toEqual(
    [
      ['ts', entry, `ts:${entry}`],
      ['ts', a, `ts:${a}`],
      ['jsx', entry, `jsx:${entry}`],
      ['jsx', b, `jsx:${b}`],
    ].sort(),
  )
  expect(visits.sort()).toEqual(
    [`ts:${entry}`, `ts:${a}`, `jsx:${entry}`, `jsx:${b}`].sort(),
  )
})

it('reruns source diagnostics on scanner hits and preserves Markdown ESM projection', () => {
  // Given equal ordinary bytes plus Markdown prose and a real ESM block.
  file('src/a.js', "import '../outside/leaf.js';")
  file('src/b.js', "import '../outside/leaf.js';")
  const leaf = file('outside/leaf.js')
  const markdown = file(
    'src/page.mdx',
    "# import 'phantom'\n\nimport '../outside/leaf.js';\n\n```js\nimport 'phantom';\n```\n",
  )
  const diagnostics: string[] = []
  __setOxcParserForTest({
    parseSync(filename) {
      diagnostics.push(filename)
      return { errors: [] }
    },
  })
  const epoch = new PreparationInputs()
  // When graph traversal shares scanner tuples across separate sources.
  const graph = buildPreparationImportGraph('src', undefined, {
    options: { cwd: root, includeMdx: true },
    inputs: epoch,
  })
  // Then diagnostic identity is not cached and raw Markdown still projects only ESM.
  expect(diagnostics.sort()).toEqual(
    [join(root, 'src/a.js'), join(root, 'src/b.js')].sort(),
  )
  expect(
    graph.requests
      .filter(({ importer }) => importer === markdown)
      .map(({ outcome }) => outcome),
  ).toEqual([expect.objectContaining({ kind: 'resolved', path: leaf })])
  __setOxcParserForTest({
    parseSync() {
      return { errors: [{ message: 'fresh diagnostic' }] }
    },
  })
  expect(() =>
    buildPreparationImportGraph('src', undefined, {
      options: { cwd: root },
      inputs: epoch,
    }),
  ).toThrow('fresh diagnostic')
})
