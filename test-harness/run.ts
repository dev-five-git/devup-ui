import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import {
  auditCoverage,
  coverageTable,
  mergeCoverage,
  writeLcov,
} from './coverage'
import { discoverTests, partitionTests, type TestGroup } from './groups'
import { CoverageError } from './lcov'
import { VerifiedSourceCoverage } from './producer-coverage'
import { captureProcess } from './producer-process'

export const requiredSources = [
  'bun.preload.ts',
  'test-harness/groups.ts',
  'test-harness/lcov.ts',
  'test-harness/coverage.ts',
  'test-harness/run.ts',
  'test-harness/preload.ts',
  'test-harness/producer-data.ts',
  'test-harness/producer-map.ts',
  'test-harness/producer-native.ts',
  'test-harness/producer-coverage.ts',
  'test-harness/producer-process.ts',
  'test-harness/producer-child.ts',
  'test-harness/producer-protocol.ts',
  'test-harness/producer-preload.ts',
  'packages/plugin-utils/src/build-admission.cts',
  'packages/plugin-utils/src/build-generation.ts',
  'packages/webpack-plugin/src/build-scope.ts',
  'packages/next-plugin/src/webpack-generation.ts',
  'packages/bun-plugin/src/plugin.ts',
  'packages/webpack-plugin/native-fixture-bundle.mjs',
] as const

export class TestProcessError extends Error {
  constructor(
    readonly group: TestGroup,
    readonly status: number | null,
  ) {
    super(`Test process ${group} failed (${status})`)
    this.name = 'TestProcessError'
  }
}

export interface TestRun {
  readonly root: string
  readonly config: string
  readonly groups: Readonly<Record<TestGroup, readonly string[]>>
  readonly required: readonly string[]
}

export function parseTestResult(output: string): {
  pass: number
  fail: number
} {
  const lines = output
    .split(/\r?\n/)
    .map((line) => line.trim())
    .join('\n')
  const pass = /^(\d+) pass$/m.exec(lines)?.[1]
  const fail = /^(\d+) fail$/m.exec(lines)?.[1]
  if (pass === undefined || fail === undefined)
    throw new CoverageError('missing child test result')
  return { pass: Number(pass), fail: Number(fail) }
}

export async function runTestGroups(
  run: TestRun,
  executable = process.execPath,
): Promise<string> {
  const coverageDirectory = join(run.root, 'coverage')
  mkdirSync(coverageDirectory, { recursive: true })
  const acceptedReport = join(coverageDirectory, 'lcov.info')
  rmSync(acceptedReport, { force: true })
  const temporary = mkdtempSync(join(coverageDirectory, 'groups-'))
  const reports = []
  let pass = 0
  let fail = 0
  try {
    for (const group of ['isolated', 'runtime'] as const) {
      const files = run.groups[group]
      if (files.length === 0)
        throw new CoverageError(`empty test group ${group}`)
      const child = await captureProcess({
        root: run.root,
        config: run.config,
        coverage: join(temporary, group),
        group,
        files,
        executable,
      })
      const output = child.value.output
      // Child totals are intermediate; only the final root summary is unprefixed.
      console.info(
        output
          .split(/\r?\n/)
          .map((line) =>
            /^\d+ (?:pass|fail)$/.test(line.trim())
              ? `[${group}] ${line}`
              : line,
          )
          .join('\n'),
      )
      if (child.value.status !== 0)
        throw new TestProcessError(group, child.value.status)
      const totals = parseTestResult(output)
      pass += totals.pass
      fail += totals.fail
      reports.push(VerifiedSourceCoverage.bind(child, run.root))
    }
    const sources = mergeCoverage(reports)
    auditCoverage(sources, run.required)
    const report = writeLcov(sources)
    writeFileSync(acceptedReport, report)
    console.info(coverageTable(sources))
    return `${pass} pass\n${fail} fail\nMerged ${sources.length} source files; 100% lines/functions per source.`
  } finally {
    rmSync(temporary, { recursive: true, force: true })
  }
}

export function rootTestRun(root: string): TestRun {
  return {
    root,
    config: join(root, 'test-harness/bunfig.toml'),
    groups: partitionTests(discoverTests(root)),
    required: requiredSources,
  }
}
