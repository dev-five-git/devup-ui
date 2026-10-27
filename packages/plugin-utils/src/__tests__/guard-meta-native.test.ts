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
const cases = [
  ['meta', 'import.meta;'],
  ['meta', 'function f(){import.meta;}'],
  ['meta', '{let meta; import.meta;}'],
  ['meta', 'function f(meta){import.meta;}'],
  ['target', 'function f(){new.target;}'],
  ['target', 'function f(target){new.target;}'],
] as const

it.each(
  cases.flatMap(([name, body]) =>
    [false, true].map((after) => ({ name, body, after })),
  ),
)(
  'has zero lexical reads for native meta properties: %j',
  ({ name, body, after }) => {
    // Given a same-named css import on either side of a real meta property.
    const declaration = `import {css as ${name}} from 'ui';`
    const source = after ? body + declaration : declaration + body
    const ast: unknown = parse(source, {
      ecmaVersion: 'latest',
      sourceType: 'module',
    })
    // When collecting both ordinary and observed facts from the native parser.
    const results = [compiledFacts(ast), compiledFacts(ast, () => {})]
    // Then keyword suffixes never become lexical binding reads, including under shadows.
    for (const facts of results) expect(facts.references).toEqual([])
  },
)

it.each([
  ['meta', 'import.meta; void meta;'],
  ['meta', 'import.meta[meta];'],
  ['target', 'function f(){new.target; void target;}'],
  ['target', 'function f(){new.target[target];}'],
] as const)(
  'retains real %s reads beside meta properties: %s',
  (name, body) => {
    // Given an identifier or computed expression that really reads the css binding.
    const source = `import {css as ${name}} from 'ui';${body}`
    const ast: unknown = parse(source, {
      ecmaVersion: 'latest',
      sourceType: 'module',
    })
    // When collecting reference facts from the real parser.
    const facts = compiledFacts(ast)
    // Then only that real read retains its exact provenance and source position.
    expect(facts.references).toEqual([
      { request: 'ui', ids: ['css'], pos: source.lastIndexOf(name) },
    ])
  },
)
