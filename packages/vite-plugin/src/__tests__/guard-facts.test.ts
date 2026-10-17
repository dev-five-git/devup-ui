import { expect, it } from 'bun:test'
import { parseAst } from 'vite'

import { compiledFacts } from '../../../plugin-utils/src/guard-facts'

it.each([
  [
    'member object',
    'import {css} from "library"; export const n=css({color:"red"}).length',
  ],
  [
    'parameter default',
    'import {css} from "library"; export function f({x=css({color:"red"})}={}){return x}',
  ],
  [
    'array default',
    'import {css} from "library"; const [x=css({color:"red"})]=[]',
  ],
  [
    'pattern default',
    'import {css} from "library"; const {x=css({color:"red"})}={}',
  ],
  [
    'computed destructure',
    'import * as DU from "library"; const {["css"]:style}=DU',
  ],
  ['direct require', 'export const n=require("library").css({color:"red"})'],
  [
    'assigned require',
    'const DU=require("library"); export const n=DU.css({color:"red"})',
  ],
  [
    'require destructure',
    'const {css:style}=require("library"); export const n=style({color:"red"})',
  ],
] as const)('records definite css when %s executes', (_name, source) => {
  const ast = parseAst(source)

  const facts = compiledFacts(ast)

  expect(
    facts.references.map(({ request, ids }) => ({ request, ids })),
  ).toContainEqual({ request: 'library', ids: ['css'] })
})

it.each([
  'const DU=require("library"); export const value=DU',
  'export const value=require("library")',
  'export const value=import("library")',
  'export function f(require){return require("library").css()}',
  'const require=other; export const value=require("library").css()',
  'export const value=require(name).css()',
  'import * as DU from "library"; export function f({css}={}){return css()}',
] as const)(
  'keeps opaque or shadowed source unclassified when %s',
  (source) => {
    const ast = parseAst(source)

    const facts = compiledFacts(ast)

    expect(facts.references).toEqual([])
  },
)

it('resolves static namespace references while preserving lexical shadowing and opaque uses', () => {
  const ast = parseAst(`import * as DU from 'library';
    import {Box as B, getTheme} from 'library';
    export function shadow(DU, {Box:B}) { return [DU.css(), B]; }
    function hoisted() { if (false) { var B; } return B; }
    const C = class DU { method() { return DU.css(); } static { const B = 1; } };
    const escaped = DU;
    const {css: style} = DU;
    let picked; ({keyframes: picked} = DU);
    const read = [DU.getTheme, DU['css'], B, getTheme];
    let name; const computed = DU[name];
    export default read;`)
  const facts = compiledFacts(ast)
  expect(facts.references.map((reference) => reference.ids)).toEqual([
    ['css'],
    ['keyframes'],
    ['getTheme'],
    ['css'],
    ['Box'],
    ['getTheme'],
  ])
})

it('retains actual forwarding names and local-export precedence for a bundler to resolve', () => {
  const facts = compiledFacts(
    parseAst(`import styled from 'alias';
    import {Box as B} from 'library';
    export {styled as Named, B as Widget};
    export {css as cls} from 'library';
    export * from 'star';
    export * as Namespace from 'library';
    export const local = 1, [first, ...rest] = [];
    export class LocalClass {}
    export default styled;`),
  )
  expect([...facts.exports]).toEqual([
    ['Named', { request: 'alias', ids: ['default'] }],
    ['Widget', { request: 'library', ids: ['Box'] }],
    ['cls', { request: 'library', ids: ['css'] }],
    ['Namespace', { request: 'library', ids: [] }],
    ['local', undefined],
    ['first', undefined],
    ['rest', undefined],
    ['LocalClass', undefined],
    ['default', { request: 'alias', ids: ['default'] }],
  ])
  expect(facts.stars).toEqual(['star'])
})

it('visits parameter defaults, catches, loop scopes and function expressions without guessing binding names', () => {
  const ast = parseAst(`import {css} from 'library';
    const local = function self({color: c = 1}, ...args) { return self; };
    try { throw 1; } catch ({message: css}) { css(); }
    for (let css of []) { css(); }
    for (let css in {}) { css(); }
    for (let css = 0; css < 1; css++) {}
    switch (1) { case 1: { let css = 0; } }
    export const result = css({color:'red'});`)
  expect(
    compiledFacts(ast).references.map((reference) => reference.ids),
  ).toEqual([['css']])
})

it('ignores data and empty public AST values rather than treating strings as imports', () => {
  expect(compiledFacts(undefined).references).toEqual([])
  expect(
    compiledFacts(
      parseAst(`export default "import {Box} from '@devup-ui/react'"`),
    ).references,
  ).toEqual([])
})
