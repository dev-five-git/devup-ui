import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { CoverageError, parseLcov } from '../../../../test-harness/lcov'
import {
  reconstructNative,
  validateNative,
} from '../../../../test-harness/producer-native'
import { captureProcess } from '../../../../test-harness/producer-process'

it('replays every native DA pair and seven actual counted functions when nested async generator and inverted public ranges occur', async () => {
  const root = mkdtempSync(join(tmpdir(), 'devup-producer-native-'))
  try {
    const config = join(root, 'bunfig.toml')
    writeFileSync(
      config,
      '[test]\ncoverage=true\ncoverageReporter=["lcov"]\ncoverageThreshold=0.0\n',
    )
    writeFileSync(
      join(root, 'source.ts'),
      'export function f() { return 1 }\nexport function h() { return 2 }\nexport function nested() { return () => 3 }\nexport async function asyncValue() { return 4 }\nexport function* generator() { yield 5 }\n',
    )
    writeFileSync(
      join(root, 'source.test.ts'),
      "import {it,expect} from 'bun:test';import {f,nested,asyncValue,generator} from './source';it('actual',async()=>{expect(f()).toBe(1);expect(nested()()).toBe(3);expect(await asyncValue()).toBe(4);expect(generator().next().value).toBe(5)})",
    )
    const result = await captureProcess({
      root,
      config,
      coverage: join(root, 'coverage'),
      group: 'isolated',
      files: ['source.test.ts'],
      executable: process.execPath,
    })
    const source = parseLcov(result.value.lcov).find(
      (item) => item.file === 'source.ts',
    )
    const capture = result.value.captures.find(
      (item) => item.url === join(root, 'source.ts'),
    )
    if (!source || !capture)
      throw new CoverageError('missing actual native fixture')
    const view = reconstructNative(capture, source)
    expect([source.foundFunctions, source.hitFunctions]).toEqual([7, 6])
    expect(capture.profile?.functions.length).toBe(9)
    expect(
      capture.blocks.some((block) => block.endOffset <= block.startOffset),
    ).toBe(true)
    expect([...view.lines].sort(([a], [b]) => a - b)).toEqual(
      [...source.lines].sort(([a], [b]) => a - b),
    )
    expect(() => validateNative(view, source)).not.toThrow()
    console.info(
      JSON.stringify({
        case: 'actual-native-seven-functions',
        pid: result.value.pid,
        code: capture.code,
        map: capture.sourceMapURL,
        profile: capture.profile,
        blocks: capture.blocks,
        functions: [source.foundFunctions, source.hitFunctions],
        lines: [...source.lines],
      }),
    )
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}, 120000)
