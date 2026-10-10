import { expect, it } from 'bun:test'

import { scanImports } from '../import-scanner'
import { oracleScannerFixtures } from './oracle-scanner-fixtures'

it.each([...oracleScannerFixtures])(
  'preserves language-sensitive runtime edges when scanning $loader: $code',
  ({ code, loader, edges }) => {
    // Given: syntax-only compilation and independently specified dependencies.
    new Bun.Transpiler({ loader }).transformSync(code)
    // When: scan the original source in its actual language, without executing it.
    const actual = scanImports(
      code,
      loader.endsWith('x'),
      loader.startsWith('t'),
    ).map(({ kind, specifier }) => `${kind}:${specifier}`)
    // Then: lexical/type-only imports disappear and runtime operands survive.
    expect(actual).toEqual([...edges])
  },
)
