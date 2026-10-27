import { expect, it } from 'bun:test'
import { parseAst } from 'vite'

import {
  compiledFacts,
  type GuardObservation,
} from '../../../plugin-utils/src/guard-facts'
import { MdxDefaultProvenanceError, validateMdxDefault } from '../mdx-default'

it.each([
  'export default function(){}',
  'const Content=function(){}; export {Content as default};',
  'const Content=()=>0; export {Content as "default"};',
])('certifies an ordinary local callable when output is %s', (source) => {
  // Given: real Vite parser output, including named and string-named defaults.
  const ast = parseAst(source)
  let proof: GuardObservation | undefined
  // When: both current-pipeline provenance boundaries inspect that output.
  compiledFacts(ast, (value) => {
    proof = value
  })
  // Then: local function expressions and local default specifiers remain valid.
  expect(proof?.defaultCallable).toBe(true)
  expect(() => validateMdxDefault(ast, source, '/content.mdx')).not.toThrow()
})

it.each([
  'export {default} from "./other.js";',
  'export {default as "default"} from "./other.js";',
  'export * as default from "./other.js";',
  'export default class Content{}',
  'function Content(){}; [Content=0]=[]; export default Content;',
  'function Content(){}; [...Content]=[]; export default Content;',
  'function Content(){}; ({...Content}={}); export default Content;',
])('refuses callable proof when compiled output is %s', (source) => {
  // Given: forwarded defaults or genuine writes through assignment/rest patterns.
  const ast = parseAst(source)
  let proof: GuardObservation | undefined
  // When: final facts and pre-extraction validation inspect the real AST.
  compiledFacts(ast, (value) => {
    proof = value
  })
  // Then: neither boundary can certify a local unmodified content component.
  expect(proof?.defaultCallable).toBe(false)
  expect(() => validateMdxDefault(ast, source, '/content.mdx')).toThrow(
    MdxDefaultProvenanceError,
  )
})

it.each([undefined, {}, { type: 'Program', body: null }])(
  'rejects an invalid parser boundary when AST is %j',
  (ast) => {
    // Given: the validator's unknown-input boundary lacks a usable Program.
    // When: that input is submitted, Then it fails closed at the real filename.
    expect(() => validateMdxDefault(ast, '', '/invalid.mdx')).toThrow(
      '/invalid.mdx:1:1:',
    )
  },
)

it('does not invent a default when malformed export children are discarded', () => {
  // Given: unknown AST input containing nonnodes and non-name export nodes.
  const ast = {
    type: 'Program',
    body: [
      null,
      {
        type: 'ExportNamedDeclaration',
        specifiers: [
          null,
          { type: 'ExportSpecifier', exported: { type: 'Identifier' } },
          { type: 'ExportSpecifier', exported: { type: 'Literal', value: 0 } },
        ],
      },
    ],
  }
  // When: prevalidation inspects it, Then no discarded child becomes a default.
  expect(() => validateMdxDefault(ast, '', '/content.mdx')).not.toThrow()
})

it('refuses import-only erasure when the Program body has a hole', () => {
  // Given: a sparse body with an otherwise ordinary side-effect import.
  const body = new Array<unknown>(2)
  body[0] = {
    type: 'ImportDeclaration',
    specifiers: [],
    source: { type: 'Literal', value: './content.mdx' },
  }
  let proof: GuardObservation | undefined
  // When: optional demand observation normalizes this unknown AST.
  compiledFacts({ type: 'Program', body }, (value) => {
    proof = value
  })
  // Then: dropping the missing child must not erase possible demand.
  expect(proof?.importOnlyRequests).toBeUndefined()
})

it.each([{ assertions: [] }, { assertions: [{ type: 'ImportAttribute' }] }])(
  'distinguishes empty legacy assertions from an asserted request: %j',
  ({ assertions }) => {
    // Given: explicit legacy assertions at the shared AST adapter boundary.
    const request = './content.mdx'
    const ast = {
      type: 'Program',
      body: [
        {
          type: 'ImportDeclaration',
          source: { type: 'Literal', value: request },
          specifiers: [],
          assertions,
        },
      ],
    }
    let proof: GuardObservation | undefined
    // When: the actual source plugin-utils observer visits the import.
    compiledFacts(ast, (value) => {
      proof = value
    })
    // Then: only the empty assertion list permits ordinary import-only erasure.
    expect(proof?.unsupportedRequests.has(request)).toBe(assertions.length > 0)
    expect(proof?.importOnlyRequests?.has(request) === true).toBe(
      assertions.length === 0,
    )
  },
)
