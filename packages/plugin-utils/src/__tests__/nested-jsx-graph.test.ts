import { readFileSync } from 'node:fs'

import { expect, it, mock } from 'bun:test'

import { __setOxcParserForTest, buildStaticImportGraph } from '../import-graph'
import { ConfigLoadError } from '../load-config'
import { createPreparedFixture } from './prepared-graph-fixture'

let root: string
const file = createPreparedFixture((directory) => {
  root = directory
})

it.each(
  [
    "const jsx = <div>{<span>import('phantom')</span> && import('./real')}{<span>require('phantom')</span> && require('./required')}</div>;",
    "const jsx = <div>{(<span>import('phantom')</span>) && import('./real')}{(<span>require('phantom')</span>) && require('./required')}</div>;",
    "const jsx = <Comp value={<span>import('phantom')</span>} next={<b>require('phantom')</b>}>require('phantom'){import('./real')}{require('./required')}</Comp>;",
  ].flatMap((code) =>
    [false, true].flatMap((enabled) =>
      [false, true].map((prepared) => ({ code, enabled, prepared })),
    ),
  ),
)(
  'resolves only real nested JSX files with diagnostics=$enabled prepared=$prepared in $code',
  async ({ code, enabled, prepared }) => {
    // Given: independently created targets and a manifest that fails if resolved.
    new Bun.Transpiler({ loader: 'tsx' }).transformSync(code)
    const entry = file('src/main.tsx', code)
    const real = file('src/real.ts', 'export const real = 1')
    const required = file('src/required.ts', 'export const required = 2')
    file('node_modules/phantom/package.json', '{')
    const programRead = mock(() => ({
      type: 'ImportExpression',
      source: { value: 'phantom' },
    }))
    const parseSync = mock(() => ({
      errors: [],
      get program() {
        return programRead()
      },
    }))
    const loaded: string[] = []
    __setOxcParserForTest(enabled ? { parseSync } : false)
    // When: traverse the real filesystem, never executing a fixture module.
    const graph = prepared
      ? await buildStaticImportGraph('src', undefined, {
          cwd: root,
          prepareSource: (filename: string) => {
            loaded.push(filename)
            return readFileSync(filename, 'utf8')
          },
        })
      : buildStaticImportGraph('src', undefined, { cwd: root })
    // Then: real edges load real files; no textual/AST phantom reaches resolution.
    expect(graph.dynamicImports.get(entry)).toEqual(new Set([real]))
    expect(graph.staticImports.get(entry)).toEqual(new Set([required]))
    expect(graph.staticImporters.get(required)).toEqual(new Set([entry]))
    expect(graph.dynamicTargets).toEqual(new Set([real]))
    expect(graph.files).toEqual([entry, real, required].sort())
    expect(graph.externalImports?.get(entry)).toEqual(new Set())
    expect(loaded.sort()).toEqual(
      prepared ? [entry, real, required].sort() : [],
    )
    expect(parseSync).toHaveBeenCalledTimes(enabled ? 3 : 0)
    expect(programRead).not.toHaveBeenCalled()
  },
)

it('rejects the poisoned manifest when an actual literal edge requests phantom', () => {
  // Given: the same poison is reachable by a real dependency, not excluded.
  file('src/main.tsx', "import('phantom')")
  file('node_modules/phantom/package.json', '{')
  __setOxcParserForTest(false)
  // When/Then: resolution of that real edge must fail.
  expect(() => buildStaticImportGraph('src', undefined, { cwd: root })).toThrow(
    ConfigLoadError,
  )
})
