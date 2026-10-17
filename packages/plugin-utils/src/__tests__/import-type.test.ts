import { expect, it } from 'bun:test'

import { scanImports } from '../import-scanner'
import { importTypeFixtures } from './import-type-fixtures'

it.each([...importTypeFixtures])(
  'elides only type imports when scanning $code',
  ({ code, jsx, edges }) => {
    // Given: syntax-valid source and independently named runtime dependencies.
    new Bun.Transpiler({ loader: jsx ? 'tsx' : 'ts' }).transformSync(code)
    // When: scan original source, never compiler output or evaluated modules.
    const actual = scanImports(code, jsx).map(
      ({ kind, specifier }) => `${kind}:${specifier}`,
    )
    // Then: types disappear while runtime typeof-import and initializers survive.
    expect(actual).toEqual([...edges])
  },
)
