import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'bun:test'

import {
  __setOxcParserForTest,
  buildStaticImportGraph,
  importedSpecifiers,
} from '../import-graph'
import { fixture } from './reservation-fixture'

afterEach(() => __setOxcParserForTest(undefined))

describe.each(['fallback', 'ast'])('%s literal require discovery', (mode) => {
  it.each([
    {
      source: 'exports.A = require("./leaf");',
      node: {
        type: 'CallExpression',
        callee: { type: 'Identifier', name: 'require' },
        arguments: [{ type: 'Literal', value: './leaf' }],
      },
      expected: ['./leaf'],
    },
    {
      source: "require /* comment */ ( /* comment */ './leaf' /* comment */ );",
      node: {
        type: 'CallExpression',
        callee: { type: 'Identifier', name: 'require' },
        arguments: [{ type: 'StringLiteral', value: './leaf' }],
      },
      expected: ['./leaf'],
    },
    {
      source: `// require('./fake')\n/* require('./fake') */\nconst text = "require('./fake')";`,
      node: { type: 'Literal', value: "require('./fake')" },
      expected: [],
    },
    {
      source: 'const text = `require("./fake")`;',
      node: { type: 'TemplateLiteral', quasis: [], expressions: [] },
      expected: [],
    },
    {
      source: 'require(name);',
      node: {
        type: 'CallExpression',
        callee: { type: 'Identifier', name: 'require' },
        arguments: [{ type: 'Identifier', name: 'name' }],
      },
      expected: [],
    },
    {
      source: 'require("./" + name);',
      node: {
        type: 'CallExpression',
        callee: { type: 'Identifier', name: 'require' },
        arguments: [{ type: 'BinaryExpression', operator: '+' }],
      },
      expected: [],
    },
    {
      source: 'require(`./leaf`);',
      node: {
        type: 'CallExpression',
        callee: { type: 'Identifier', name: 'require' },
        arguments: [{ type: 'TemplateLiteral', quasis: [], expressions: [] }],
      },
      expected: [],
    },
    {
      source: 'require("./fake", extra);',
      node: {
        type: 'CallExpression',
        callee: { type: 'Identifier', name: 'require' },
        arguments: [
          { type: 'Literal', value: './fake' },
          { type: 'Identifier', name: 'extra' },
        ],
      },
      expected: [],
    },
    {
      source: 'object /* comment */ . /* comment */ require("./fake");',
      node: {
        type: 'CallExpression',
        callee: { type: 'MemberExpression', property: { name: 'require' } },
        arguments: [{ type: 'Literal', value: './fake' }],
      },
      expected: [],
    },
    {
      source: 'require?.("./fake");',
      node: {
        type: 'CallExpression',
        optional: true,
        callee: { type: 'Identifier', name: 'require' },
        arguments: [{ type: 'Literal', value: './fake' }],
      },
      expected: [],
    },
    {
      source: 'new require("./fake");',
      node: {
        type: 'NewExpression',
        callee: { type: 'Identifier', name: 'require' },
        arguments: [{ type: 'Literal', value: './fake' }],
      },
      expected: [],
    },
    {
      source: 'otherrequire("./fake"); $require("./fake");',
      node: {
        type: 'CallExpression',
        callee: { type: 'Identifier', name: '$require' },
        arguments: [{ type: 'Literal', value: './fake' }],
      },
      expected: [],
    },
  ])(
    'discovers only direct static calls in $source',
    ({ source, node, expected }) => {
      // Given
      __setOxcParserForTest(
        mode === 'ast'
          ? { parseSync: () => ({ program: { body: [node] } }) }
          : false,
      )
      // When
      const specifiers = importedSpecifiers('entry.js', source)
      // Then
      expect(specifiers).toEqual(expected)
    },
  )

  it('records literal require as static while retaining dynamic import edges', () => {
    // Given
    using fs = fixture()
    fs.write('src/entry.js', 'require("./leaf"); import("./lazy");')
    fs.write('src/leaf.js', 'export const leaf = 1;')
    fs.write('src/lazy.js', 'export const lazy = 1;')
    __setOxcParserForTest(
      mode === 'ast'
        ? {
            parseSync: (filename) => ({
              body: filename.endsWith('entry.js')
                ? [
                    {
                      type: 'CallExpression',
                      callee: { type: 'Identifier', name: 'require' },
                      arguments: [{ type: 'Literal', value: './leaf' }],
                    },
                    { type: 'ImportExpression', source: { value: './lazy' } },
                  ]
                : [],
            }),
          }
        : false,
    )
    // When
    const graph = buildStaticImportGraph(join(fs.root, 'src'))
    // Then
    expect(graph.staticImports.get(join(fs.root, 'src/entry.js'))).toEqual(
      new Set([join(fs.root, 'src/leaf.js')]),
    )
    expect(graph.dynamicImports.get(join(fs.root, 'src/entry.js'))).toEqual(
      new Set([join(fs.root, 'src/lazy.js')]),
    )
  })
})
