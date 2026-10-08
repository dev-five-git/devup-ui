import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, beforeEach, expect, it, spyOn } from 'bun:test'

import { CoverageError, parseLcov } from '../../../../test-harness/lcov'
import { prepareTests } from '../../../../test-harness/preload'
import {
  parseTestResult,
  runTestGroups,
  TestProcessError,
} from '../../../../test-harness/run'

let restore: () => void
beforeEach(() => {
  const output = spyOn(console, 'info').mockImplementation(() => undefined)
  const errors = spyOn(console, 'error').mockImplementation(() => undefined)
  restore = () => {
    output.mockRestore()
    errors.mockRestore()
  }
})
afterEach(() => restore())

function fixture(assertion = "expect(value()).toBe('runtime')") {
  const root = mkdtempSync(join(tmpdir(), 'devup-test-harness-'))
  mkdirSync(join(root, 'tests'))
  const config = join(root, 'bunfig.toml')
  writeFileSync(
    config,
    '[test]\ncoverage=true\ncoverageReporter=["lcov"]\ncoverageThreshold=0.0\ncoverageSkipTestFiles=true\n',
  )
  writeFileSync(
    join(root, 'source.ts'),
    'export function value() {return process.env.DEVUP_TEST_GROUP}\n',
  )
  writeFileSync(
    join(root, 'tests/a.test.ts'),
    "import {expect,it} from 'bun:test';import {value} from '../source';it('isolated',()=>expect(value()).toBe('isolated'))",
  )
  writeFileSync(
    join(root, 'tests/b.test.ts'),
    `import {expect,it} from 'bun:test';import {value} from '../source';it('runtime',()=>{${assertion}})`,
  )
  return {
    root,
    config,
    groups: { isolated: ['tests/a.test.ts'], runtime: ['tests/b.test.ts'] },
    required: ['source.ts'],
  }
}

it('runs both real groups separately when the root harness is invoked', async () => {
  const run = fixture()
  try {
    const summary = await runTestGroups(run)
    expect(parseTestResult(summary)).toEqual({ pass: 2, fail: 0 })
    const files = parseLcov(
      readFileSync(join(run.root, 'coverage/lcov.info'), 'utf8'),
    ).map((source) => source.file)
    expect(files).toContain('source.ts')
    expect(
      files.some((file) => file.endsWith('test-harness/producer-preload.ts')),
    ).toBe(true)
    expect(files.length).toBe(2)
  } finally {
    rmSync(run.root, { recursive: true, force: true })
  }
})

it('propagates a real second-process assertion failure when the first group succeeds', async () => {
  const run = fixture("expect(value()).toBe('isolated')")
  try {
    await expect(runTestGroups(run)).rejects.toThrow(TestProcessError)
  } finally {
    rmSync(run.root, { recursive: true, force: true })
  }
})

it('rejects missing source coverage when both processes pass', async () => {
  const run = fixture()
  try {
    await expect(
      runTestGroups({ ...run, required: ['missing.ts'] }),
    ).rejects.toThrow(CoverageError)
  } finally {
    rmSync(run.root, { recursive: true, force: true })
  }
})

it('refuses an empty configured group when the harness would omit its tests', async () => {
  const run = fixture()
  try {
    await expect(
      runTestGroups({
        ...run,
        groups: { isolated: [], runtime: run.groups.runtime },
      }),
    ).rejects.toThrow(CoverageError)
  } finally {
    rmSync(run.root, { recursive: true, force: true })
  }
})

it('propagates the launch error when the test executable is absent', async () => {
  const run = fixture()
  try {
    await expect(
      runTestGroups(run, join(run.root, 'missing-executable')),
    ).rejects.toThrow(Error)
  } finally {
    rmSync(run.root, { recursive: true, force: true })
  }
})

it.each(['', '1 pass', '0 fail'])(
  'refuses incomplete child totals when output is %s',
  (output) => {
    expect(() => parseTestResult(output)).toThrow(CoverageError)
  },
)

it('returns success exactly once when root preload finishes the real synthetic groups', async () => {
  const run = fixture()
  try {
    expect(await prepareTests(undefined, run)).toBe(0)
  } finally {
    rmSync(run.root, { recursive: true, force: true })
  }
})

it('returns failure instead of retrying discovery when root preload sees a failed child', async () => {
  const run = fixture("expect(value()).toBe('isolated')")
  try {
    expect(await prepareTests(undefined, run)).toBe(1)
  } finally {
    rmSync(run.root, { recursive: true, force: true })
  }
})
