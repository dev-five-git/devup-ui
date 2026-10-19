import { join } from 'node:path'

import { afterEach, expect, it } from 'bun:test'

import {
  __setOxcParserForTest,
  buildStaticImportGraph,
  createModuleResolver,
  importedSpecifiers,
} from '../import-graph'
import { fixture } from './reservation-fixture'

afterEach(() => __setOxcParserForTest(undefined))

it.each([
  [`"import './fake'"`, "import './real'"],
  [`'import "./fake"'`, 'import "./real"'],
  [`"import { value } from './fake'"`, "import { value } from './real'"],
  [`'import value from "./fake"'`, 'import value from "./real"'],
  [`"export { value } from './fake'"`, "export { value } from './real'"],
  [`'export * from "./fake"'`, 'export * from "./real"'],
  [`"import('./fake')"`, "import('./real')"],
  [`'import("./fake")'`, 'import("./real")'],
  [String.raw`"escaped \" import './fake'"`, "import './real'"],
  [String.raw`'escaped \' import "./fake"'`, 'import "./real"'],
  [String.raw`"backslash \\"`, "import './real'"],
  ['`import "./fake"`', "import './real'"],
  ['`export * from "./fake"`', 'export * from "./real"'],
  ['`import("./fake")`', 'import("./real")'],
])('ignores a quoted %s while retaining genuine %s', (snippet, genuine) => {
  // Given
  __setOxcParserForTest(false)
  const source = `const snippet = ${snippet};\n${genuine};`
  // When
  const specifiers = importedSpecifiers('entry.ts', source)
  // Then
  expect(specifiers).toEqual(['./real'])
})

it('retains real static, reexport and dynamic graph edges around comments', () => {
  // Given
  using fs = fixture()
  __setOxcParserForTest(false)
  fs.write(
    'src/entry.js',
    [
      '// import "./fake"',
      '/* export * from "./fake"; import("./fake") */',
      'import /* comment */ "./static";',
      'export { value } /* comment */ from "./reexport";',
      'import(/* comment */ "./dynamic");',
    ].join('\n'),
  )
  for (const name of ['static', 'reexport', 'dynamic', 'fake'])
    fs.write(`src/${name}.js`, 'export const value = 1;')
  // When
  const graph = buildStaticImportGraph(join(fs.root, 'src'))
  // Then
  expect(graph.staticImports.get(join(fs.root, 'src/entry.js'))).toEqual(
    new Set(
      ['static', 'reexport'].map((name) => join(fs.root, `src/${name}.js`)),
    ),
  )
  expect(graph.dynamicImports.get(join(fs.root, 'src/entry.js'))).toEqual(
    new Set([join(fs.root, 'src/dynamic.js')]),
  )
})

it('ignores quoted snippets when the optional AST parser fails', () => {
  // Given
  __setOxcParserForTest({
    parseSync() {
      throw new SyntaxError('parser unavailable for this input')
    },
  })
  // When
  const specifiers = importedSpecifiers(
    'entry.ts',
    `const snippet = "import './fake'"; import './real';`,
  )
  // Then
  expect(specifiers).toEqual(['./real'])
})

it('preserves JSON string contents when resolving commented tsconfig aliases', () => {
  // Given
  using fs = fixture()
  fs.write(
    'tsconfig.json',
    `{
      // JSON strings are configuration, not source snippets.
      "compilerOptions": {
        "baseUrl": ".",
        "paths": { "@/*": ["src/*"], },
      },
    }`,
  )
  fs.write('src/real.js', 'export const value = 1;')
  const resolver = createModuleResolver({
    cwd: fs.root,
    tsconfigPath: join(fs.root, 'tsconfig.json'),
  })
  // When
  const module = resolver('@/real', join(fs.root, 'src/entry.js'))
  // Then
  expect(module?.path).toBe(join(fs.root, 'src/real.js'))
})
