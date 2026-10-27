import { expect, it } from 'bun:test'

import { compiledFacts, type GuardObservation } from '../guard-facts'

const request = './Description.mdx'
const ordinary = {
  type: 'ImportDeclaration',
  specifiers: [],
  source: { type: 'Literal', value: request },
}
const program = (body: readonly unknown[]) => ({ type: 'Program', body })
function observation(ast: unknown): GuardObservation | undefined {
  let result: GuardObservation | undefined
  compiledFacts(ast, (value) => {
    result = value
  })
  return result
}

it('observes complete ordinary import-only demand without changing public facts', () => {
  // Given a complete Program with explicit empty specifier lists.
  const ast = program([
    ordinary,
    {
      ...ordinary,
      source: { type: 'Literal', value: './other.js' },
      attributes: [],
      assertions: [],
    },
  ])
  // When optional proof accompanies the existing facts processing.
  const facts = compiledFacts(ast)
  let proof: GuardObservation | undefined
  const observed = compiledFacts(ast, (value) => {
    proof = value
  })
  // Then exact requests are witnessed, but no callable default is invented.
  expect(observed).toEqual(facts)
  expect(proof?.importOnlyRequests).toEqual(new Set([request, './other.js']))
  expect(proof?.defaultCallable).toBe(false)
})

it.each([
  ['absent AST', undefined],
  ['missing body', { type: 'Program' }],
  ['empty body', program([])],
  ['discarded directives', { ...program([ordinary]), directives: [null] }],
  ['malformed directives', { ...program([ordinary]), directives: null }],
  ['malformed body', { type: 'Program', body: {} }],
  ['discarded body child', program([ordinary, null])],
  ['primitive body child', program([ordinary, 0])],
  [
    'sparse body',
    program(Object.assign(new Array<unknown>(2), { 0: ordinary })),
  ],
  ['unknown statement', program([ordinary, { type: 'Unknown' }])],
  ['not a Program', { type: 'BlockStatement', body: [ordinary] }],
  [
    'missing specifiers',
    program([{ type: 'ImportDeclaration', source: ordinary.source }]),
  ],
  ['type-only import', program([{ ...ordinary, importKind: 'type' }])],
  ['malformed specifiers', program([{ ...ordinary, specifiers: {} }])],
  ['discarded specifier', program([{ ...ordinary, specifiers: [null] }])],
  ['sparse specifiers', program([{ ...ordinary, specifiers: new Array(1) }])],
  [
    'surviving default',
    program([
      { ...ordinary, specifiers: [{ type: 'ImportDefaultSpecifier' }] },
    ]),
  ],
  ['missing source', program([{ type: 'ImportDeclaration', specifiers: [] }])],
  [
    'identifier source',
    program([{ ...ordinary, source: { type: 'Identifier', name: request } }]),
  ],
  [
    'missing literal value',
    program([{ ...ordinary, source: { type: 'Literal' } }]),
  ],
  [
    'nonstring source',
    program([{ ...ordinary, source: { type: 'Literal', value: 1 } }]),
  ],
  ['null attributes', program([{ ...ordinary, attributes: null }])],
  ['malformed attributes', program([{ ...ordinary, attributes: {} }])],
  ['discarded attribute', program([{ ...ordinary, attributes: [null] }])],
  ['sparse attributes', program([{ ...ordinary, attributes: new Array(1) }])],
  [
    'attribute node',
    program([{ ...ordinary, attributes: [{ type: 'ImportAttribute' }] }]),
  ],
  ['malformed assertions', program([{ ...ordinary, assertions: false }])],
  ['discarded assertion', program([{ ...ordinary, assertions: [null] }])],
  ['unrelated statement', program([ordinary, { type: 'EmptyStatement' }])],
] as const)('does not erase demand for %s', (_name, ast) => {
  // Given incomplete/unsupported AST input, When the same normalizer visits it,
  // Then discarded fields never become a proof of absent demand.
  expect(observation(ast)?.importOnlyRequests).toBeUndefined()
})

it('does not turn a changed request into the original demand witness', () => {
  const proof = observation(
    program([
      { ...ordinary, source: { type: 'Literal', value: './changed.mdx' } },
    ]),
  )
  expect(proof?.importOnlyRequests?.has(request)).toBe(false)
})
