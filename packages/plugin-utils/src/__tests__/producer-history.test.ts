import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { auditCoverage, mergeCoverage } from '../../../../test-harness/coverage'
import { CoverageError, parseLcov } from '../../../../test-harness/lcov'
import { VerifiedSourceCoverage } from '../../../../test-harness/producer-coverage'
import {
  reconstructNative,
  validateNative,
} from '../../../../test-harness/producer-native'
import {
  captureProcess,
  type ProducerResult,
} from '../../../../test-harness/producer-process'

it('retains opposite native branch histories when the public precise deltas are both zero after their baselines', async () => {
  const root = mkdtempSync(join(tmpdir(), 'devup-producer-history-'))
  try {
    const config = join(root, 'bunfig.toml')
    writeFileSync(
      config,
      '[test]\npreload=["./early.ts"]\ncoverage=true\ncoverageReporter=["lcov"]\ncoverageThreshold=0.0\n',
    )
    writeFileSync(
      join(root, 'source.ts'),
      'export function choice(active: boolean) {\n  if (active) {\n    return 1\n  }\n  return 2\n}\n',
    )
    writeFileSync(
      join(root, 'early.ts'),
      "import {choice} from './source';choice(process.env.DEVUP_TEST_GROUP==='isolated')\n",
    )
    writeFileSync(
      join(root, 'source.test.ts'),
      "import {it,expect} from 'bun:test';it('actual',()=>expect(1).toBe(1))",
    )
    const results: ProducerResult[] = []
    for (const group of ['isolated', 'runtime'] as const) {
      results.push(
        await captureProcess({
          root,
          config,
          coverage: join(root, group),
          group,
          files: ['source.test.ts'],
          executable: process.execPath,
        }),
      )
    }
    const observations = results.map((result) => {
      const capture = result.value.captures.find(
        (item) => item.url === join(root, 'source.ts'),
      )
      const source = parseLcov(result.value.lcov).find(
        (item) => item.file === 'source.ts',
      )
      if (!capture || !source || !capture.profile)
        throw new CoverageError('missing actual history fixture')
      return { capture, source, result }
    })
    const first = observations[0]
    const second = observations[1]
    if (!first || !second)
      throw new CoverageError('missing actual history pair')
    expect(
      first.capture.profile?.functions.every((fn) =>
        fn.ranges.every((range) => range.count === 0),
      ),
    ).toBe(true)
    expect(
      second.capture.profile?.functions.every((fn) =>
        fn.ranges.every((range) => range.count === 0),
      ),
    ).toBe(true)
    expect([...first.source.lines]).not.toEqual([...second.source.lines])
    expect(() =>
      validateNative(
        reconstructNative(first.capture, first.source),
        second.source,
      ),
    ).toThrow(CoverageError)
    const sources = mergeCoverage(
      results.map((result) => VerifiedSourceCoverage.bind(result, root)),
    )
    expect(() => auditCoverage(sources, ['source.ts'])).not.toThrow()
    console.info(
      JSON.stringify({
        case: 'actual-prebaseline-history',
        observations: observations.map(({ capture, source, result }) => ({
          pid: result.value.pid,
          code: capture.code,
          map: capture.sourceMapURL,
          profile: capture.profile,
          blocks: capture.blocks,
          functions: [source.foundFunctions, source.hitFunctions],
          lines: [...source.lines],
        })),
      }),
    )
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}, 120000)

it('refuses actual f-g versus f-h producer inputs when raw source, anonymous totals and range keys cannot prove compatibility', async () => {
  const root = mkdtempSync(join(tmpdir(), 'devup-producer-effective-'))
  try {
    const config = join(root, 'bunfig.toml')
    const raw =
      'export function f() { return 1 }\nexport function g() { return 2 }\n'
    writeFileSync(
      config,
      '[test]\npreload=["./transform.ts"]\ncoverage=true\ncoverageReporter=["lcov"]\ncoverageThreshold=0.0\n',
    )
    writeFileSync(join(root, 'source.ts'), raw)
    writeFileSync(
      join(root, 'transform.ts'),
      `import {plugin} from 'bun';await plugin({name:'actual-effective-input',setup(build){build.onLoad({filter:/source\\.ts$/},()=>({contents:process.env.DEVUP_TEST_GROUP==='isolated'?${JSON.stringify(raw)}:${JSON.stringify(raw.replace('g()', 'h()'))},loader:'ts'}))}})`,
    )
    writeFileSync(
      join(root, 'a.test.ts'),
      "import {it,expect} from 'bun:test';import {f,g} from './source';it('actual',()=>{expect(f()).toBe(1);expect(g()).toBe(2)})",
    )
    writeFileSync(
      join(root, 'b.test.ts'),
      "import {it,expect} from 'bun:test';import {f} from './source';it('actual',()=>expect(f()).toBe(1))",
    )
    const results: ProducerResult[] = []
    for (const [group, file] of [
      ['isolated', 'a.test.ts'],
      ['runtime', 'b.test.ts'],
    ] as const) {
      results.push(
        await captureProcess({
          root,
          config,
          coverage: join(root, group),
          group,
          files: [file],
          executable: process.execPath,
        }),
      )
    }
    const observations = results.map((result) => {
      const capture = result.value.captures.find(
        (item) => item.url === join(root, 'source.ts'),
      )
      const source = parseLcov(result.value.lcov).find(
        (item) => item.file === 'source.ts',
      )
      if (!capture || !source)
        throw new CoverageError('missing actual effective-input fixture')
      return { capture, source }
    })
    const first = observations[0]
    const second = observations[1]
    if (!first || !second)
      throw new CoverageError('missing actual effective-input pair')
    expect(readFileSync(join(root, 'source.ts'), 'utf8')).toBe(raw)
    expect(observations.map((item) => item.source.foundFunctions)).toEqual([
      2, 2,
    ])
    expect([...first.source.lines.keys()]).toEqual([
      ...second.source.lines.keys(),
    ])
    expect(
      first.capture.profile?.functions
        .map((fn) => fn.ranges[0])
        .map((range) => [range?.startOffset, range?.endOffset]),
    ).toEqual(
      second.capture.profile?.functions
        .map((fn) => fn.ranges[0])
        .map((range) => [range?.startOffset, range?.endOffset]),
    )
    expect(first.capture.code).not.toBe(second.capture.code)
    expect(() =>
      mergeCoverage(
        results.map((result) => VerifiedSourceCoverage.bind(result, root)),
      ),
    ).toThrow(CoverageError)
    console.info(
      JSON.stringify({
        case: 'actual-effective-input-countercase',
        raw,
        observations: observations.map(({ capture, source }) => ({
          code: capture.code,
          map: capture.sourceMapURL,
          profile: capture.profile,
          blocks: capture.blocks,
          functions: [source.foundFunctions, source.hitFunctions],
          lines: [...source.lines],
        })),
      }),
    )
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}, 120000)
