import { join } from 'node:path'

import { expect, it, mock } from 'bun:test'

import { __setOxcParserForTest, buildStaticImportGraph } from '../import-graph'
import { jsxBoundaryFixtures } from './jsx-boundary-fixtures'
import { createPreparedFixture } from './prepared-graph-fixture'

let root: string
const file = createPreparedFixture((directory) => {
  root = directory
})

it.each([...jsxBoundaryFixtures])(
  'preserves expected filesystem graph edges when scanning $code',
  async ({ code, jsx, edges }) => {
    // Given: real leaves and a poisoned phantom dependency under a unique fixture root.
    const entry = file(`src/main.${jsx ? 'tsx' : 'ts'}`, code)
    for (const name of ['real', 'required', 'inside', 'attribute', 'child'])
      file(`src/${name}.ts`)
    file('node_modules/phantom/package.json', '{')
    const programRead = mock(() => ({
      type: 'ImportDeclaration',
      source: { value: 'phantom' },
    }))
    const parseSync = mock(() => ({
      errors: [],
      get program() {
        return programRead()
      },
    }))
    const options = {
      cwd: root,
      prepareSource: (filename: string) =>
        jsx && filename === entry ? code : undefined,
    }
    // When: traverse the same prepared filesystem graph with diagnostics disabled and injected.
    __setOxcParserForTest(false)
    const disabled = await buildStaticImportGraph('src', undefined, options)
    __setOxcParserForTest({ parseSync })
    const injected = await buildStaticImportGraph('src', undefined, options)
    // Then: each expected edge resolves independently, with no AST reads or phantom resolution.
    expect(injected).toEqual(disabled)
    const kinds: readonly ('static' | 'dynamic')[] = ['static', 'dynamic']
    for (const kind of kinds) {
      const expected = new Set(
        edges
          .filter((edge) => edge.startsWith(`${kind}:`))
          .map((edge) =>
            join(root, 'src', `${edge.slice(kind.length + 3)}.ts`),
          ),
      )
      expect(
        { static: injected.staticImports, dynamic: injected.dynamicImports }[
          kind
        ].get(entry),
      ).toEqual(expected)
    }
    expect(injected.externalImports?.get(entry)).toEqual(new Set())
    expect(parseSync).toHaveBeenCalledTimes(injected.files.length)
    expect(programRead).not.toHaveBeenCalled()
  },
)
