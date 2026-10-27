import { expect, it } from 'bun:test'
import { parseAst } from 'vite'

import { overwrittenCallableCases } from '../../../plugin-utils/src/__tests__/guard-callable-negative-cases'
import { intactCallableCases } from '../../../plugin-utils/src/__tests__/guard-callable-positive-cases'
import {
  compiledFacts,
  type GuardObservation,
} from '../../../plugin-utils/src/guard-facts'
import { sourceFacts } from '../compiled-facts'

const cases = [
  ...intactCallableCases,
  ...overwrittenCallableCases,
  ['empty', ''],
  ['side effects', 'import "one"; import "two"; import "one";'],
  ['directive and side effect', '"use strict"; import "one";'],
  ['empty and side effect', '; import "one";'],
  ['named import', 'import {Box as B} from "ui"; export const view=B;'],
  [
    'namespace alias',
    'import * as DU from "ui"; const Alias=(DU); Alias.css();',
  ],
  ['parenthesized member', 'import * as DU from "ui"; (DU).css();'],
  ['parenthesized callable', 'const Content=(()=>0); export default Content;'],
  [
    'empty forwarding request',
    'const Content=()=>0; export {Content as default} from "";',
  ],
  ['require alias', 'const DU=require("ui"); const alias=DU; alias.css({});'],
  ['require shadow', 'function f(require){return require("ui").css();}'],
  ['require variable shadow', 'const require=other; require("ui").css();'],
  ['opaque computed', 'import * as DU from "ui"; DU[key]();'],
  ['computed read', 'import {css} from "ui"; const result=obj[css];'],
  ['literal numeric member', 'import * as DU from "ui"; DU[0]; DU[true];'],
  [
    'nested pattern',
    'import {css} from "ui"; function f({[css]:[x=css,...rest]}={}){return css();}',
  ],
  [
    'assignment pattern',
    'import * as DU from "ui"; let picked; ({keyframes:picked}=DU);',
  ],
  [
    'forwarding',
    'import styled from "alias"; export {styled as Named}; export {css as cls} from "ui"; export * from "star"; export * as Namespace from "ui"; export const local=1,[first,...rest]=[]; export class LocalClass{}; export default styled;',
  ],
  [
    'scoped declarations',
    'import {css} from "ui"; function f(){if(false){var css;} return css;} try{}catch(css){css();} for(let css of []){css();} for(let css in {}){css();} css();',
  ],
  [
    'class shadows',
    'import {css} from "ui"; const C=class css {method(){return css;} static {let css;}}; css();',
  ],
  [
    'attributes',
    'import data from "json" with {type:"json"}; export {default} from "other" with {type:"json"};',
  ],
  [
    'astral offset',
    `const astral="${String.fromCodePoint(0x1f600)}"; import * as DU from "ui"; DU.css();`,
  ],
  [
    'raw lone surrogate request',
    `import * as DU from "${String.fromCharCode(0xd800)}"; DU.css();`,
  ],
] as const

it.each(cases)(
  'matches the existing public-parser facts for %s',
  (_name, code) => {
    const ast = parseAst(code)
    const expected = compiledFacts(ast)

    const actual = sourceFacts(code)

    expect(actual).toEqual(expected)
  },
)

it.each(cases)(
  'matches the existing public-parser provenance observation for %s',
  (_name, code) => {
    const ast = parseAst(code)
    let expected: GuardObservation | undefined
    const expectedFacts = compiledFacts(ast, (value) => {
      expected = value
    })
    let actual: GuardObservation | undefined

    const actualFacts = sourceFacts(code, (value) => {
      actual = value
    })

    expect({ facts: actualFacts, observation: actual }).toEqual({
      facts: expectedFacts,
      observation: expected,
    })
  },
)

it('rejects malformed final JavaScript with a located parser error', () => {
  const code = 'export {'

  const parse = () => sourceFacts(code)

  expect(parse).toThrow(/1:\d+/)
})
