import { expect, it } from 'bun:test'

import { scanImports } from '../import-scanner'
import { jsxBoundaryFixtures } from './jsx-boundary-fixtures'

it.each([...jsxBoundaryFixtures])(
  'keeps independent edges when scanning $code',
  ({ code, jsx, edges }) => {
    // Given: syntax-checked source, never evaluated or used to derive expectations.
    new Bun.Transpiler({ loader: jsx ? 'tsx' : 'ts' }).transformSync(code)
    // When: scan the original source.
    const actual = scanImports(code, jsx).map(
      ({ kind, specifier }) => `${kind}:${specifier}`,
    )
    // Then: only the fixture's independently named edges survive.
    expect(actual).toEqual([...edges])
  },
)
