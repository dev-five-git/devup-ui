import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { CoverageError, parseLcov } from '../../../../test-harness/lcov'
import { VerifiedSourceCoverage } from '../../../../test-harness/producer-coverage'
import { decodeMap, mappedLines } from '../../../../test-harness/producer-map'
import { reconstructNative } from '../../../../test-harness/producer-native'
import { captureProcess } from '../../../../test-harness/producer-process'

it('strictly binds complementary reloads without discarding any original evidence', async () => {
  // Given: the same conditional module executes client/server/client.
  const root = mkdtempSync(join(tmpdir(), 'devup-reload-union-'))
  try {
    const config = join(root, 'bunfig.toml')
    writeFileSync(
      config,
      '[test]\ncoverage=true\ncoverageReporter=["lcov"]\ncoverageThreshold=0.0\n',
    )
    writeFileSync(
      join(root, 'source.ts'),
      "export const value = process.env.RELOAD_SIDE === 'server' ? 'server' : 'client'\n",
    )
    writeFileSync(
      join(root, 'source.test.ts'),
      "import {it,expect} from 'bun:test';it('complementary',async()=>{process.env.RELOAD_SIDE='client';expect((await import('./source?first')).value).toBe('client');process.env.RELOAD_SIDE='server';expect((await import('./source?middle')).value).toBe('server');process.env.RELOAD_SIDE='client';expect((await import('./source?last')).value).toBe('client');Bun.gc(true)})",
    )
    const result = await captureProcess({
      root,
      config,
      coverage: join(root, 'coverage'),
      group: 'isolated',
      files: ['source.test.ts'],
      executable: process.execPath,
    })
    const captures = result.value.captures.filter(
      (item) => item.url === join(root, 'source.ts'),
    )
    const latest = [...captures]
      .sort((a, b) => Number(a.scriptId) - Number(b.scriptId))
      .at(-1)
    const source = parseLcov(result.value.lcov).find(
      (item) => item.file === 'source.ts',
    )
    if (!latest || !source)
      throw new CoverageError('missing actual reload evidence')
    expect(captures.length).toBe(3)
    expect(captures.every((item) => !item.profile)).toBe(true)
    for (const item of captures)
      expect(item.blocks.at(-1)).toEqual({
        startOffset: 0,
        endOffset: latest.code.length - 1,
        count: 1,
        hasExecuted: true,
      })
    // When: unchanged strict binding consumes the actually issued receipt.
    const bound = VerifiedSourceCoverage.bind(result, root).find(
      (item) => item.file === source.file,
    )
    // Then: every native row and function survives; contradictory evidence still fails closed.
    expect([...(bound?.lines ?? [])]).toEqual([...source.lines])
    expect([bound?.foundFunctions, bound?.hitFunctions]).toEqual([1, 1])
    const lines = new Map<number, number>()
    const map = decodeMap(latest.code, latest.sourceMapURL)
    for (const block of latest.blocks.slice(0, -1))
      for (const line of mappedLines(map, block))
        lines.set(
          line,
          (lines.get(line) ?? 0) + Number(block.hasExecuted || block.count > 0),
        )
    const base = { ...source, lines }
    expect(reconstructNative(latest, base, [latest, latest]).lines).toEqual(
      lines,
    )
    for (const invalid of [
      { ...latest, sourceMapURL: '' },
      { ...latest, scriptType: 'unknown' },
      { ...latest, blocks: latest.blocks.slice(1) },
      { ...latest, blocks: [...latest.blocks].reverse() },
      {
        ...latest,
        blocks: latest.blocks.map((block, index) =>
          index === latest.blocks.length - 1 ? { ...block, count: 2 } : block,
        ),
      },
    ])
      expect(() => reconstructNative(latest, base, [invalid, latest])).toThrow(
        CoverageError,
      )
    expect(latest.code.length).toBeLessThan(1e9)
    const bad = { ...source, lines: new Map([[1, 1e9]]) }
    expect(() => reconstructNative(latest, bad, captures)).toThrow(
      CoverageError,
    )
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}, 120000)
