import { expect, it } from 'bun:test'

import { CoverageError, parseLcov } from '../../../../test-harness/lcov'
import type {
  BasicBlock,
  ScriptCapture,
} from '../../../../test-harness/producer-data'
import { decodeMap, mappedLines } from '../../../../test-harness/producer-map'
import {
  reconstructNative,
  validateNative,
} from '../../../../test-harness/producer-native'

const map = Buffer.from(
  JSON.stringify({
    version: 3,
    sources: ['/source.ts'],
    sourcesContent: [
      "'use client'\r\nimport { useEffect, useLayoutEffect } from 'react'\r\n\r\nexport const useSafeEffect: typeof useEffect =\r\n  typeof window === 'undefined' ? useEffect : useLayoutEffect\r\n",
    ],
    mappings: 'AAAA;AACA;AAEO,aAAM,gBACX,OAAO,WAAW,cAAc,YAAY',
    names: [],
  }),
).toString('base64')
const sourceMapURL = `data:application/json;base64,${map}`
const code = `"use client";\nimport { useEffect, useLayoutEffect } from "react";\nexport const useSafeEffect = typeof window === "undefined" ? useEffect : useLayoutEffect;\n\n//# sourceMappingURL=${sourceMapURL}\n`
const server = code.indexOf('useEffect :')
const client = code.indexOf(': useLayoutEffect;')
const end = code.length - 1

/** One evaluation of the module: the server or the client branch ran. */
function evaluation(
  scriptId: string,
  side: 'server' | 'client',
): ScriptCapture {
  const block = (
    startOffset: number,
    endOffset: number,
    executed: boolean,
  ): BasicBlock => ({
    startOffset,
    endOffset,
    count: Number(executed),
    hasExecuted: executed,
  })
  return {
    scriptId,
    url: '/source.ts',
    scriptType: 'module',
    sourceMapURL,
    code,
    blocks: [
      block(server, server + 'useEffect'.length, side === 'server'),
      block(client, client + ': useLayoutEffect'.length, side === 'client'),
      block(client + ': useLayoutEffect'.length, end, true),
      block(0, server - 1, true),
      block(0, end, true),
    ],
    profile: {
      scriptId,
      url: '/source.ts',
      functions: [
        {
          isBlockCoverage: true,
          ranges: [{ startOffset: 0, endOffset: code.length, count: 1 }],
        },
        {
          isBlockCoverage: true,
          ranges: [{ startOffset: 0, endOffset: end, count: 1 }],
        },
      ],
    },
  }
}

function linesOf(blocks: readonly BasicBlock[]) {
  const lines = new Map<number, number>()
  const decoded = decodeMap(code, sourceMapURL)
  for (const block of blocks.slice(0, -1))
    for (const line of mappedLines(decoded, block))
      lines.set(
        line,
        (lines.get(line) ?? 0) + Number(block.hasExecuted || block.count > 0),
      )
  return lines
}

const reloads = [
  evaluation('1', 'client'),
  evaluation('2', 'server'),
  evaluation('3', 'client'),
]
const latest = reloads[2]
const [parsed] = parseLcov(
  'SF:source.ts\nFNF:1\nFNH:1\nDA:1,1\nLF:1\nLH:1\nend_of_record\n',
)

it('binds framed reloads that ran complementary branches to the union of their evidence', () => {
  // Given: Bun reports one executed-line row for three evaluations of the same module.
  if (!latest || !parsed) throw new CoverageError('missing reload fixture')
  const union = reloads[0]?.blocks.map((block, index) => {
    const executed = reloads.some((item) => item.blocks[index]?.hasExecuted)
    return { ...block, hasExecuted: executed, count: Number(executed) }
  })
  if (!union) throw new CoverageError('missing reload fixture')
  const source = { ...parsed, lines: linesOf(union) }
  // When: the newest evaluation is reconstructed together with its reloads.
  const view = reconstructNative(latest, source, reloads)
  // Then: every row, including the line of the other branch, matches the report.
  expect(() => validateNative(view, source)).not.toThrow()
  expect([...view.lines]).toEqual([...source.lines])
  expect(view.functions).toHaveLength(1)
  expect(view.code).toBe(code)
})

it('keeps a single framed evaluation exactly as observed', () => {
  // Given: a module evaluated once.
  if (!latest || !parsed) throw new CoverageError('missing reload fixture')
  const source = { ...parsed, lines: linesOf(latest.blocks) }
  // When / Then: neither an absent nor a one-element reload list changes the rows.
  expect([...reconstructNative(latest, source).lines]).toEqual([
    ...source.lines,
  ])
  expect([...reconstructNative(latest, source, [latest]).lines]).toEqual([
    ...source.lines,
  ])
})

it('keeps the rows of the newest evaluation when an older one reported fewer blocks', () => {
  // Given: an older evaluation never ran the branches, so it only has the outer blocks.
  if (!latest || !parsed) throw new CoverageError('missing reload fixture')
  const older = { ...latest, blocks: latest.blocks.slice(2) }
  const source = { ...parsed, lines: linesOf(latest.blocks) }
  // When / Then: the newest evaluation's rows survive unchanged.
  expect([...reconstructNative(latest, source, [older, latest]).lines]).toEqual(
    [...source.lines],
  )
})

it.each([
  ['from another file', { url: '/other.ts' }],
  ['with other code', { code: `${code}\n` }],
  ['with another source map', { sourceMapURL: 'data:,' }],
  ['of another script type', { scriptType: 'unknown' }],
])('rejects a framed reload %s', (_name, change) => {
  // Given: a reload that is not an evaluation of the same script.
  if (!latest || !parsed) throw new CoverageError('missing reload fixture')
  const source = { ...parsed, lines: linesOf(latest.blocks) }
  // When / Then: the evidence is refused instead of merged.
  expect(() =>
    reconstructNative(latest, source, [latest, { ...latest, ...change }]),
  ).toThrow(CoverageError)
})
