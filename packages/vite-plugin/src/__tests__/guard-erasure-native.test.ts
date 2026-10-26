import { expect, it } from 'bun:test'
import { parseAst } from 'vite'

import {
  compiledFacts,
  type GuardObservation,
} from '../../../plugin-utils/src/guard-facts'

function observe(ast: unknown): GuardObservation | undefined {
  let result: GuardObservation | undefined
  compiledFacts(ast, (value) => {
    result = value
  })
  return result
}

it('witnesses only complete bare-import programs from the actual Vite parser', () => {
  // Given the actual native parser output for an erased final root.
  const ast = parseAst("import './Description.mdx'; import './provider.js';")
  // When the same facts seam supplies optional demand evidence.
  const proof = observe(ast)
  // Then exact bare requests are witnessed without inventing a callable default.
  expect(proof?.importOnlyRequests).toEqual(
    new Set(['./Description.mdx', './provider.js']),
  )
  expect(proof?.defaultCallable).toBe(false)
  expect(
    observe(parseAst("import './Description.mdx'; export const retained=0;"))
      ?.importOnlyRequests,
  ).toBeUndefined()
})

it('does not convert discarded children or hidden directives into native erasure proof', () => {
  const ast = parseAst("import './Description.mdx';")
  expect(
    observe({ ...ast, body: [...ast.body, null] })?.importOnlyRequests,
  ).toBeUndefined()
  expect(
    observe({ ...ast, directives: [null] })?.importOnlyRequests,
  ).toBeUndefined()
})
