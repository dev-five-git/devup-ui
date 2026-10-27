import { join } from 'node:path'

import { expect, it, mock } from 'bun:test'

import { __setOxcParserForTest, buildStaticImportGraph } from '../import-graph'
import { ConfigLoadError } from '../load-config'
import { importTypeFixtures } from './import-type-fixtures'
import { createPreparedFixture } from './prepared-graph-fixture'

let root: string
const file = createPreparedFixture((directory) => {
  root = directory
})

it.each(
  importTypeFixtures
    .filter(({ edges }) => !edges.includes('dynamic:./types'))
    .flatMap((fixture) =>
      [false, true].map((prepared) => ({ ...fixture, prepared })),
    ),
)(
  'ignores poisoned type modules with prepared=$prepared when scanning $code',
  async ({ code, jsx, edges, prepared }) => {
    // Given: real runtime leaves and a malformed manifest for type-only requests.
    const source = code
      .replaceAll('./types', 'phantom')
      .replaceAll('./other-types', 'phantom')
    const compiled = new Bun.Transpiler({
      loader: jsx ? 'tsx' : 'ts',
    }).transformSync(source)
    const entry = file(`src/main.${jsx ? 'tsx' : 'ts'}`, source)
    for (const name of ['real', 'required', 'body']) file(`src/${name}.ts`)
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
    const options = {
      cwd: root,
      prepareSource: (filename: string) =>
        prepared && filename === entry ? compiled : undefined,
    }
    // When: traverse the actual filesystem with diagnostics disabled and injected.
    __setOxcParserForTest(false)
    const disabled = await buildStaticImportGraph('src', undefined, options)
    __setOxcParserForTest({ parseSync })
    const injected = await buildStaticImportGraph('src', undefined, options)
    // Then: only fixture-derived runtime edges load; no type/AST phantom is resolved.
    expect(injected).toEqual(disabled)
    const kinds: readonly ('static' | 'dynamic')[] = ['static', 'dynamic']
    for (const kind of kinds) {
      expect(
        { static: injected.staticImports, dynamic: injected.dynamicImports }[
          kind
        ].get(entry),
      ).toEqual(
        new Set(
          edges
            .filter((edge) => edge.startsWith(`${kind}:`))
            .map((edge) =>
              join(root, 'src', `${edge.slice(kind.length + 3)}.ts`),
            ),
        ),
      )
    }
    expect(injected.externalImports?.get(entry)).toEqual(new Set())
    expect(parseSync).toHaveBeenCalledTimes(injected.files.length)
    expect(programRead).not.toHaveBeenCalled()
  },
)

it('reaches the poisoned manifest when typeof import is genuinely runtime code', () => {
  // Given: the same poisoned dependency, in both a type and a runtime expression.
  file(
    'src/main.ts',
    "type X = typeof import('phantom'); const x = typeof import('phantom');",
  )
  file('node_modules/phantom/package.json', '{')
  __setOxcParserForTest(false)
  // When/Then: runtime dependency resolution must still reach the malformed manifest.
  expect(() => buildStaticImportGraph('src', undefined, { cwd: root })).toThrow(
    ConfigLoadError,
  )
})
