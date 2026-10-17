import { expect, it, mock } from 'bun:test'

import { __setOxcParserForTest, buildStaticImportGraph } from '../import-graph'
import { createPreparedFixture } from './prepared-graph-fixture'

let root: string
const file = createPreparedFixture((directory) => {
  root = directory
})

it.each([false, true])(
  'discovers the real template interpolation import with parser enabled %s',
  (enabled) => {
    // Given: valid source and a real leaf; the injected AST describes the prior divergence.
    const entry = file(
      'src/main.ts',
      "export const result = `${import('./leaf')}`",
    )
    const leaf = file('src/leaf.ts')
    const parseSync = mock(() => ({
      program: { type: 'ImportExpression', source: { value: './leaf' } },
      errors: [],
    }))
    __setOxcParserForTest(enabled ? { parseSync } : false)
    // When: build the real filesystem graph.
    const graph = buildStaticImportGraph('src', undefined, { cwd: root })
    // Then: interpolation code loads the leaf regardless of parser presence.
    expect(graph.dynamicImports.get(entry)).toEqual(new Set([leaf]))
    if (enabled) expect(parseSync).toHaveBeenCalledTimes(2)
  },
)

it.each([false, true])(
  'ignores contradictory AST edges with optional diagnostics %s',
  (enabled) => {
    // Given: literal code edges and poisoned textual/AST dependencies.
    const entry = file(
      'src/main.tsx',
      [
        'export * from "./static"; export {value} from "./reexport"; import "./side-effect";',
        'const value = `text import("phantom") ${import("./dynamic")} ${require("./required")}`;',
        'const nested = `${ `inner ${import("./nested")}` }`;',
        'const regex = /[`"\']import("phantom")/; const division = 1 / import("./division");',
        'const jsx = <div title="require phantom">import("phantom"){require("./attribute")}</div>;',
        '/* import("phantom") */ import(name); require(name); import("phantom" + name);',
      ].join('\n'),
    )
    const staticTargets = [
      'static',
      'reexport',
      'side-effect',
      'required',
      'attribute',
    ].map((name) => file(`src/${name}.ts`))
    const dynamicTargets = ['dynamic', 'nested', 'division'].map((name) =>
      file(`src/${name}.ts`),
    )
    file('node_modules/phantom/package.json', '{')
    const parseSync = mock(() => ({
      errors: [],
      program: { type: 'ImportExpression', source: { value: 'phantom' } },
    }))
    __setOxcParserForTest(enabled ? { parseSync } : false)
    // When: resolve the fixture through the actual graph path.
    const graph = buildStaticImportGraph('src', undefined, { cwd: root })
    // Then: expected paths come only from the fixture's real literal dependencies.
    expect(graph.staticImports.get(entry)).toEqual(new Set(staticTargets))
    expect(graph.dynamicImports.get(entry)).toEqual(new Set(dynamicTargets))
    expect(graph.externalImports?.get(entry)).toEqual(new Set())
    expect(parseSync).toHaveBeenCalledTimes(enabled ? graph.files.length : 0)
  },
)
