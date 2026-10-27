import { createRequire } from 'node:module'
import { resolve } from 'node:path'

import { expect, it } from 'bun:test'

import { compiledFacts } from '../guard-facts'

const require = createRequire(
  resolve(import.meta.dir, '../../../../apps/landing/package.json'),
)
const parser: unknown = require('next/dist/compiled/acorn')
if (
  !parser ||
  typeof parser !== 'object' ||
  !('parse' in parser) ||
  typeof parser.parse !== 'function'
)
  throw new TypeError('Installed Webpack parser must expose parse')
const parse = parser.parse

it.each([
  ['named', 'import {css as c} from "ui"; c({})', ['css']],
  ['default', 'import styled from "ui"; styled("div")', ['default']],
  ['namespace', 'import * as DU from "ui"; DU.css({})', ['css']],
  ['literal member', 'import * as DU from "ui"; DU["css"]({})', ['css']],
  ['require member', 'require("ui").css({})', ['css']],
  [
    'const alias',
    'const DU=require("ui"); const alias=DU; alias.css({})',
    ['css'],
  ],
  ['destructure', 'const {css:c,...rest}=require("ui");', ['css']],
  ['assignment', 'let c; ({css:c}=require("ui"));', ['css']],
  [
    'nested member',
    'import * as DU from "ui"; DU.stylex.create({})',
    ['stylex', 'create'],
  ],
] as const)(
  'retains definite %s origins from real parser output',
  (_name, source, ids) => {
    // Given: actual JavaScript accepted by Webpack's installed parser.
    const ast: unknown = parse(source, {
      ecmaVersion: 'latest',
      sourceType: 'module',
    })
    // When: build-time references are collected.
    const facts = compiledFacts(ast)
    // Then: the definitely named surface retains its request and export path.
    expect(facts.references).toContainEqual({
      request: 'ui',
      ids: [...ids],
      pos: expect.any(Number),
      ...(_name.startsWith('require') ||
      ['const alias', 'destructure', 'assignment'].includes(_name)
        ? { kind: 'require-call' }
        : {}),
    })
  },
)

it.each([
  'import * as DU from "ui"; function f(DU){return DU.css()}',
  'import * as DU from "ui"; { let DU; DU.css() }',
  'function f(require){return require("ui").css()}',
  'const DU=require("ui"); DU[unknown]()',
  'const DU=import("ui"); DU.css()',
  'require("ui", extra).css()',
  'class C { method(){var DU; return DU.css()} }',
] as const)(
  'does not invent definite exports for opaque or shadowed source: %s',
  (source) => {
    const ast: unknown = parse(source, {
      ecmaVersion: 'latest',
      sourceType: 'module',
    })
    const facts = compiledFacts(ast)
    expect(facts.references).toEqual([])
  },
)

it('retains public forwarding and local export distinctions from native ASTs', () => {
  const ast: unknown = parse(
    `
    import {css} from 'ui';
    export {css as style};
    export {Box as Widget} from 'components';
    export * from 'star';
    export * as namespace from 'namespace';
    export const {x:y=css,...rest} = {};
    export function local() {}
    export default css;
  `,
    { ecmaVersion: 'latest', sourceType: 'module' },
  )
  const facts = compiledFacts(ast)
  expect([...facts.exports]).toEqual([
    ['style', { request: 'ui', ids: ['css'] }],
    ['Widget', { request: 'components', ids: ['Box'] }],
    ['namespace', { request: 'namespace', ids: [] }],
    ['y', undefined],
    ['rest', undefined],
    ['local', undefined],
    ['default', { request: 'ui', ids: ['css'] }],
  ])
  expect(facts.stars).toEqual(['star'])
})

it('finds pattern expressions without treating pattern binding names as reads', () => {
  const ast: unknown = parse(
    `
    import {css} from 'ui'; import * as DU from 'ui';
    function f({[css]: value=css}, [head=css,,...tail], ...args) {
      try { throw 1 } catch ({message=css}) { var local; }
      for (let item of []) { const {nested:{x=css}} = {}; }
      const C = class Named { static { let value; } };
      (()=>css)(); (function named(a=css){return a})();
      ({member: css}).member; DU[css];
    }
  `,
    { ecmaVersion: 'latest', sourceType: 'module' },
  )
  const facts = compiledFacts(ast)
  expect(facts.references.length).toBeGreaterThan(5)
  expect(
    facts.references.every(
      (ref) => ref.request === 'ui' && ref.ids[0] === 'css',
    ),
  ).toBe(true)
})

it.each([undefined, null, 1, 'source', {}])(
  'rejects non-AST boundary input %s without manufactured facts',
  (input) => {
    expect(compiledFacts(input)).toEqual({
      references: [],
      exports: new Map(),
      stars: [],
    })
  },
)
