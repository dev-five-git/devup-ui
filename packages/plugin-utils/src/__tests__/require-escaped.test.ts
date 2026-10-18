import { afterEach, describe, expect, it } from 'bun:test'

import { __setOxcParserForTest, importedSpecifiers } from '../import-graph'

afterEach(() => __setOxcParserForTest(undefined))

describe.each(['fallback', 'ast'])('%s escaped require discovery', (mode) => {
  it.each([
    [String.raw`"./\x41.js"`, './A.js'],
    [String.raw`'./\u0042.js'`, './B.js'],
    [String.raw`'.\/\u{41}.js'`, './A.js'],
    [String.raw`".\u002f\x42.js"`, './B.js'],
    [String.raw`"./escaped\"quote.js"`, './escaped"quote.js'],
    [String.raw`'./escaped\'quote.js'`, "./escaped'quote.js"],
    [String.raw`"./back\\slash.js"`, './back\\slash.js'],
    ['"./line\\\ncontinued.js"', './linecontinued.js'],
  ])(
    'decodes static literal %s to its physical specifier',
    (literal, value) => {
      // Given
      const source = `require(${literal});`
      __setOxcParserForTest(
        mode === 'ast'
          ? {
              parseSync: () => ({
                type: 'CallExpression',
                callee: { type: 'Identifier', name: 'require' },
                arguments: [{ type: 'Literal', value }],
              }),
            }
          : false,
      )
      // When
      const specifiers = importedSpecifiers('entry.js', source)
      // Then
      expect(specifiers).toEqual([value])
    },
  )
})

it.each([
  String.raw`require("./\x4G.js");`,
  String.raw`require('./\u00ZZ.js');`,
  String.raw`require("./\u{110000}.js");`,
  String.raw`require("./\u{}.js");`,
  String.raw`require("./\x41.js" + name);`,
  String.raw`require(name);`,
  'require(`./\\u0041.js`);',
  String.raw`const text = "escaped \" require('./\x41.js')";`,
  String.raw`const text = 'escaped \' require("./\u0042.js")';`,
  String.raw`// require("./\x41.js")`,
])('ignores malformed, nonliteral or quoted escaped call %s', (source) => {
  // Given
  __setOxcParserForTest(false)
  // When
  const specifiers = importedSpecifiers('entry.js', source)
  // Then
  expect(specifiers).toEqual([])
})
