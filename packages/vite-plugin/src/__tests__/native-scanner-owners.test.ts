import { expect, it } from 'bun:test'

import { oracleScannerFixtures } from '../../../plugin-utils/src/__tests__/oracle-scanner-fixtures'
import { scanImports } from '../../../plugin-utils/src/import-scanner'

it.each(oracleScannerFixtures)(
  'finds only runtime edges in V-owned $loader source: $code',
  ({ code, loader, edges }) => {
    // Given real language-specific lexical corpus data, not another lane's tests.
    const jsx = loader === 'jsx' || loader === 'tsx'
    const typescript = loader === 'ts' || loader === 'tsx'
    // When V scans its own dependency inputs.
    const references = scanImports(code, jsx, typescript)
    // Then the observed runtime edges exclude masked text and type imports.
    expect(
      references.map(({ kind, specifier }) => `${kind}:${specifier}`),
    ).toEqual([...edges])
  },
)

it.each([
  [
    'interface Shape { value: import("./types").Value }\n' +
      'let a: import("./types").Value, b: import("./types").Value;\n' +
      'export default 1; import("./real");',
    false,
    ['dynamic:./real'],
  ],
  [
    '// import("ignored")\n/* require("ignored") */\n' +
      'const re = /[\\/]import("ignored")/gi; const n = 8 / 2;\n' +
      'const text = `escaped\\` ${`nested ${require("./nested")}`}`;\n' +
      'import "./side"; export * from "./export"; import("./lazy", {with:{type:"json"}});',
    false,
    ['static:./nested', 'static:./side', 'static:./export', 'dynamic:./lazy'],
  ],
  [
    'const node = <><A title="require(ignored)" value={require("./attribute")}>text<B />{import("./child")}</A></>;',
    true,
    ['static:./attribute', 'dynamic:./child'],
  ],
  [
    'async /* gap */ function f() { return /require("ignored")/; }\n' +
      'class C { method() { return import("./method") } }\n' +
      'if (ok) {} /import("ignored")/.test(value);',
    false,
    ['dynamic:./method'],
  ],
] as const)(
  'preserves real edges through lexical containers in %s',
  (code, jsx, edges) => {
    // Given lexical containers which resemble imports.
    // When scanning the complete source.
    const references = scanImports(code, jsx)
    // Then only executable import expressions survive masking.
    expect(
      references.map(({ kind, specifier }) => `${kind}:${specifier}`),
    ).toEqual([...edges])
  },
)
