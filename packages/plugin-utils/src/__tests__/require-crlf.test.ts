import { afterEach, describe, expect, it } from 'bun:test'

import { __setOxcParserForTest, importedSpecifiers } from '../import-graph'

afterEach(() => __setOxcParserForTest(undefined))

describe.each(['fallback', 'ast'])('%s CRLF require discovery', (mode) => {
  it.each(['"', "'"])(
    'follows physical literals with CRLF continuations inside %s quotes',
    (quote) => {
      // Given
      const source = ['A', 'B']
        .map((name) => `require(${quote}./\\\r\n${name}.js${quote});`)
        .join('\n')
      __setOxcParserForTest(
        mode === 'ast'
          ? {
              parseSync: () => ({
                body: ['A', 'B'].map((name) => ({
                  type: 'CallExpression',
                  callee: { type: 'Identifier', name: 'require' },
                  arguments: [{ type: 'Literal', value: `./${name}.js` }],
                })),
              }),
            }
          : false,
      )
      // When
      const specifiers = importedSpecifiers('entry.js', source)
      // Then
      expect(specifiers).toEqual(['./A.js', './B.js'])
    },
  )
})

it.each([
  'require("./\r\nA.js");',
  "require('./\r\nB.js');",
  'const snippet = "require(\'./\\\r\nA.js\')";',
  'const snippet = \'require("./\\\r\nB.js")\';',
])('ignores invalid or quoted CRLF payload %s', (source) => {
  // Given
  __setOxcParserForTest(false)
  // When
  const specifiers = importedSpecifiers('entry.js', source)
  // Then
  expect(specifiers).toEqual([])
})
