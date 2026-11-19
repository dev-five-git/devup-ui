import assert from 'node:assert/strict'
import { test } from 'node:test'

import { cases } from '../validation-cases.mjs'
import {
  countScenarioCalls,
  evaluate,
  parseDeclarations,
  parseOutputMembers,
  report,
  UNVALIDATED,
} from './coverage.mjs'

const dts = `/* tslint:disable */
export class Output {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Get the code
     */
    readonly code: string;
    readonly css: string | undefined;
}

export function getCss(file_num: number | null | undefined, import_main_css: boolean): string;

/**
 * Docs mentioning export function fake(a: string) stay out.
 */
export function setPrefix(prefix?: string | null): void;

export function isDebug(): boolean;
`

const state = (exports, config, css) => ({
  exports: Object.fromEntries(exports.map((name) => [name, {}])),
  config: Object.fromEntries(config.map((name) => [name, {}])),
  interface: {},
  css: Array.from({ length: css }, () => ({})),
})

test('parses exported functions with their parameter names', () => {
  assert.deepEqual(parseDeclarations(dts), {
    getCss: ['file_num', 'import_main_css'],
    setPrefix: ['prefix'],
    isDebug: [],
  })
})

test('parses the Output members as the scenarios label them', () => {
  assert.deepEqual(parseOutputMembers(dts), [
    'free',
    'Symbol(Symbol.dispose)',
    'code',
    'css',
  ])
  assert.deepEqual(parseOutputMembers('export function a(): void;'), [])
})

test('counts steps and the observers each step snapshots', () => {
  const snapshot = state(['exportSheet'], ['isDebug'], 2)
  const calls = countScenarioCalls([
    { kind: 'note', label: 'ignored' },
    { kind: 'start', state: snapshot },
    { kind: 'step', label: 'setPrefix', before: snapshot, after: snapshot },
    { kind: 'step', label: 'setPrefix', before: snapshot, after: snapshot },
  ])
  assert.deepEqual(calls, {
    setPrefix: 2,
    exportSheet: 4,
    isDebug: 4,
    getThemeInterface: 4,
    getCss: 8,
  })
})

const fixture = (overrides = {}) => ({
  declarations: {
    getCss: ['file_num'],
    hasDevupUI: ['import_aliases'],
    isDebug: [],
  },
  members: ['code'],
  calls: { getCss: 1, hasDevupUI: 1, isDebug: 1, 'live-code': 1 },
  cases: [{ fn: 'getCss', parameter: 'file_num', index: 0 }],
  ...overrides,
})

test('passes when every export and member is called and every argument rejected', () => {
  const result = evaluate(fixture())
  assert.equal(result.ok, true)
  assert.deepEqual(result.problems, [])
  assert.equal(result.exports, 4)
  assert.equal(result.called, 4)
  assert.equal(result.positions, 2)
  assert.equal(result.validated, 1)
  assert.deepEqual(
    result.rows.map((row) => [row.name, row.called]),
    [
      ['getCss', 1],
      ['hasDevupUI', 1],
      ['isDebug', 1],
      ['Output.code', 1],
    ],
  )
  const markdown = report(result)
  assert.match(markdown, /4\/4 exports and `Output` members called/)
  assert.match(markdown, /1\/2 argument positions with rejection cases/)
  assert.match(markdown, /\| `getCss` \| 1 \| file_num: 1 \|/)
  assert.match(
    markdown,
    /\| `hasDevupUI` \| 1 \| import_aliases: unvalidated \|/,
  )
  assert.match(markdown, /\| `isDebug` \| 1 \| - \|/)
  assert.doesNotMatch(markdown, /Problems/)
})

test('fails on uncalled exports, unvalidated or moved arguments and stale cases', () => {
  const result = evaluate(
    fixture({
      declarations: { getCss: ['index', 'file_num'], isDebug: [] },
      calls: { getCss: 1 },
      cases: [
        { fn: 'getCss', parameter: 'file_num', index: 0 },
        { fn: 'removed', parameter: 'value', index: 0 },
      ],
    }),
  )
  assert.equal(result.ok, false)
  assert.deepEqual(result.problems, [
    'getCss.index: no rejection case',
    'getCss.file_num: declared at 1, validated at 0',
    'isDebug: never called',
    'Output.code: never called',
    'removed.value: rejection case for an undeclared parameter',
    'hasDevupUI.import_aliases: stale UNVALIDATED entry',
  ])
  const markdown = report(result)
  assert.match(markdown, /index: unvalidated, file_num: 1/)
  assert.match(markdown, /- isDebug: never called/)
})

test('exempts only the documented unvalidated parameters', () => {
  const result = evaluate(
    fixture({
      declarations: { hasDevupUI: ['filename', 'import_aliases'] },
      members: [],
      calls: { hasDevupUI: 1 },
      cases: [{ fn: 'hasDevupUI', parameter: 'filename', index: 0 }],
    }),
  )
  assert.deepEqual(result.problems, [])
  assert.ok(UNVALIDATED['hasDevupUI.import_aliases'])
  assert.match(report(result), /import_aliases: unvalidated/)
})

test('the validation oracle covers every documented argument position', () => {
  const positions = new Set(cases.map((item) => `${item.fn}.${item.parameter}`))
  assert.equal(positions.size, 55)
  for (const key of Object.keys(UNVALIDATED)) assert.ok(!positions.has(key))
})
