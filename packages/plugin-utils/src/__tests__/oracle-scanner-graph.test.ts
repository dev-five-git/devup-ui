import { join } from 'node:path'

import { expect, it, mock } from 'bun:test'

import { __setOxcParserForTest, buildStaticImportGraph } from '../import-graph'
import { ConfigLoadError } from '../load-config'
import { oracleScannerFixtures } from './oracle-scanner-fixtures'
import { createPreparedFixture } from './prepared-graph-fixture'

let root: string
const file = createPreparedFixture((directory) => {
  root = directory
})

it.each(
  oracleScannerFixtures.flatMap((fixture) =>
    [false, true].map((prepared) => ({ ...fixture, prepared })),
  ),
)(
  'resolves independent real paths without poisoned false edges when $loader prepared=$prepared: $code',
  async ({ code, loader, edges, prepared }) => {
    // Given: real source-language filenames, live leaves, and poisoned phantom manifests.
    const source = code.replaceAll('./types', 'phantom')
    const entry = file(`src/main.${loader}`, source)
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
    // When: scan raw or same-language prepared source with diagnostics disabled/injected.
    const options = { cwd: root }
    const preparedOptions = {
      ...options,
      prepareSource: (filename: string) =>
        filename === entry ? source : undefined,
    }
    __setOxcParserForTest(false)
    const disabled = prepared
      ? await buildStaticImportGraph('src', undefined, preparedOptions)
      : buildStaticImportGraph('src', undefined, options)
    __setOxcParserForTest({ parseSync })
    const injected = prepared
      ? await buildStaticImportGraph('src', undefined, preparedOptions)
      : buildStaticImportGraph('src', undefined, options)
    // Then: full graph parity, independent expected paths, and zero AST authority.
    expect(injected).toEqual(disabled)
    for (const kind of ['static', 'dynamic'] as const) {
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

it('keeps JavaScript comparison operands under a real compiled Markdown filename', async () => {
  // Given: compiled JSX under the original custom extension, never a renamed shim.
  const entry = file('src/page.mdown', '# source')
  const real = file('src/real.ts')
  file('node_modules/phantom/package.json', '{')
  const code =
    "const x = a < import('./real') > (b); const node = <div>import('phantom')</div>;"
  const programRead = mock(() => ({}))
  const parseSync = mock(() => ({
    errors: [],
    get program() {
      return programRead()
    },
  }))
  const options = {
    cwd: root,
    includeMdx: ['.mdown'],
    prepareSource: (filename: string) =>
      filename === entry ? code : undefined,
  }
  // When: traverse compiled JS/JSX with both optional diagnostic modes.
  __setOxcParserForTest(false)
  const disabled = await buildStaticImportGraph('src', undefined, options)
  __setOxcParserForTest({ parseSync })
  const injected = await buildStaticImportGraph('src', undefined, options)
  // Then: the real path and dynamic operand survive; quoted JSX text stays masked.
  expect(injected).toEqual(disabled)
  expect(injected.dynamicImports.get(entry)).toEqual(new Set([real]))
  expect(injected.staticImports.get(entry)).toEqual(new Set())
  expect(parseSync).toHaveBeenCalledTimes(injected.files.length)
  expect(programRead).not.toHaveBeenCalled()
})

it('reaches a poisoned manifest when an ambiguous type-named binding is a value', () => {
  // Given: the value spelling adjacent to the type-only as countercases.
  file('src/main.ts', "import {type as renamed} from 'phantom';")
  file('node_modules/phantom/package.json', '{')
  __setOxcParserForTest(false)
  // When/Then: a genuine value dependency still reaches manifest validation.
  expect(() => buildStaticImportGraph('src', undefined, { cwd: root })).toThrow(
    ConfigLoadError,
  )
})
