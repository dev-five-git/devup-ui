import { expect, it } from 'bun:test'

import { scanImports } from '../import-scanner'

it.each([
  {
    code: 'export const result = `${import("./leaf")}`',
    edges: ['dynamic:./leaf'],
  },
  {
    code: 'export const result = `${require("./leaf")}`',
    edges: ['static:./leaf'],
  },
  {
    code: 'const s = `text ${ {a: `nested ${import("./a")}`, b: require("./b")} } tail import("fake")`',
    edges: ['dynamic:./a', 'static:./b'],
  },
  {
    code: 'const s = tag`escaped \\` \\${import("fake")} ${ /[`"}]/.test("}") ? import("./a") : require("./b") } tail`',
    edges: ['dynamic:./a', 'static:./b'],
  },
  {
    code: 'const s = `${ (() => { /* } ` */ // } `\n return import("./a") })() }`',
    edges: ['dynamic:./a'],
  },
  {
    code: 'const s = `${ <div title={require("./b")}>import("fake"){import("./a")}</div> }`',
    edges: ['dynamic:./a', 'static:./b'],
  },
  {
    code: 'const a = 10 / import("./a"); const b = /[`"\']import("fake")/.test(a); const c = require("./b") / 2;',
    edges: ['dynamic:./a', 'static:./b'],
  },
  {
    code: 'const x = {a: 1} / import("./a"); x++ / require("./b"); --x / import("./c");',
    edges: ['dynamic:./a', 'dynamic:./c', 'static:./b'],
  },
  {
    code: 'if (x) { y() } /import("fake")/.test(x); while (x) /import("fake")/.test(x); import("./a")',
    edges: ['dynamic:./a'],
  },
  {
    code: 'const x = "import(\\"fake\\")"; /* require("fake") */ // import("fake")\n const y = <><div title="import fake">require("fake")</div><span>{require("./b")}</span></>;',
    edges: ['static:./b'],
  },
  {
    code: 'export*from"./a"; export{x}from"./b"; export * as ns from "./c"; import"./d";',
    edges: ['static:./a', 'static:./b', 'static:./c', 'static:./d'],
  },
  {
    code: 'import /*x*/ {x} /*y*/ from "./a"; require /*x*/ ( /*y*/ "./b" /*z*/ ); import("./c", {with:{type:"json"}});',
    edges: ['dynamic:./c', 'static:./a', 'static:./b'],
  },
  {
    code: 'import(name); require(name); import("./a" + name); require("./b" + name); import(`./c`); object.require("fake"); object?.import("fake");',
    edges: [],
  },
  {
    code: 'import(`${"fake"}`); require(`text ${"fake"}`); import(`text ${import("./a")}`);',
    edges: ['dynamic:./a'],
  },
  {
    code: 'import type {A} from "fake"; export type*from"fake"; import{type A,type B,}from"fake"; export{type A}from"fake"; import {type A,b} from "./a"; import Default,{type A}from"./b"; import type from "./c"; import{type as renamed}from"./d";',
    edges: ['static:./a', 'static:./b', 'static:./c', 'static:./d'],
  },
  {
    code: String.raw`import "./\x61"; export{x}from'./\u0062'; require('./\u{63}'); import("./quo\"te"); import('./quo\'te'); import('./back\\slash');`,
    edges: [
      'dynamic:./quo"te',
      "dynamic:./quo'te",
      'dynamic:./back\\slash',
      'static:./a',
      'static:./b',
      'static:./c',
    ].sort(),
  },
  {
    code: 'const s = `${ "}\\"" + /[}\\/`]/gi.test(x) + import("./a") } after`;',
    edges: ['dynamic:./a'],
  },
  {
    code: 'const s = `${ x in /import("fake")/ ? import("./a") : "" }`;',
    edges: ['dynamic:./a'],
  },
  {
    code: 'import from from "./a"; import {from} from "./b"; export {from as x} from "./c";',
    edges: ['static:./a', 'static:./b', 'static:./c'],
  },
  {
    code: 'import type, {value} from "./a"; import type, * as values from "./b"; import {"string-name" as name} from "./c"; export {name as "string-name"} from "./d";',
    edges: ['static:./a', 'static:./b', 'static:./c', 'static:./d'],
  },
  {
    code: 'const fn = function() {} / import("./a"); const C = class {} / require("./b");',
    edges: ['dynamic:./a', 'static:./b'],
  },
  {
    code: 'try {} finally {} /import("fake")/.test(x); class C {} /require("fake")/.test(x); function f() {} /import("fake")/.test(x); import("./a");',
    edges: ['dynamic:./a'],
  },
  {
    code: 'async function f() {} /import("fake")/.test(x); const fn = async function() {} / import("./a"); const g = function(a={x:1}, b=()=>{}) { return function() {} / require("./b") };',
    edges: ['dynamic:./a', 'static:./b'],
  },
  {
    code: 'export async /* marker */ function f() {} /import("phantom")/.test(value); import("./real");',
    edges: ['dynamic:./real'],
  },
  {
    code: 'const fn = async /*marker*/ function() {} / import("./real")',
    edges: ['dynamic:./real'],
  },
  {
    code: 'async/*marker*//*next*/function f() {} /import("phantom")/.test(value); import("./real");',
    edges: ['dynamic:./real'],
  },
  {
    code: 'const value = async /*marker*/ / import("./real") / 2; /*next*/ function f() {}',
    edges: ['dynamic:./real'],
  },
])('discovers only fixture-derived edges in $code', ({ code, edges }) => {
  // Given: independent expected edge sets, including text that resembles imports.
  // When: scan without executing source or consulting a parser.
  const actual = scanImports(code, true)
    .map((edge) => `${edge.kind}:${edge.specifier}`)
    .sort()
  // Then: only literal dependencies in code are edges.
  expect(actual).toEqual([...edges])
})

it('preserves TypeScript angle assertions without interpreting them as JSX', () => {
  const code = 'const value = <number>input / 2; import("./leaf")'
  expect(scanImports(code, false)).toEqual([
    { kind: 'dynamic', specifier: './leaf' },
  ])
})

it('decodes escaped control characters and line continuations without evaluation', () => {
  const code = String.raw`require('a\n\r\t\b\f\v\0\z'); import('a\
b'); import('a\
b');`
  expect(scanImports(code, false)).toEqual([
    { kind: 'static', specifier: 'a\n\r\t\b\f\v\0z' },
    { kind: 'dynamic', specifier: 'ab' },
    { kind: 'dynamic', specifier: 'ab' },
  ])
})
