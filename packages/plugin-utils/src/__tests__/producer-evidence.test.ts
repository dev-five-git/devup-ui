import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it, spyOn } from 'bun:test'

import { CoverageError, parseLcov } from '../../../../test-harness/lcov'
import { VerifiedSourceCoverage } from '../../../../test-harness/producer-coverage'
import { captureProcess } from '../../../../test-harness/producer-process'
import { parseTestResult, runTestGroups } from '../../../../test-harness/run'

it('accepts exact function union when independent real children execute complementary functions of the same effective source', async () => {
  const root = mkdtempSync(join(tmpdir(), 'devup-producer-positive-'))
  const output = spyOn(console, 'info').mockImplementation(() => undefined)
  try {
    const config = join(root, 'bunfig.toml')
    writeFileSync(
      config,
      '[test]\ncoverage=true\ncoverageReporter=["lcov"]\ncoverageThreshold=0.0\n',
    )
    writeFileSync(
      join(root, 'source.ts'),
      'export function first() { return 1 }\nexport function second() { return 2 }\n',
    )
    for (const [name, fn, value] of [
      ['a', 'first', 1],
      ['b', 'second', 2],
    ] as const) {
      writeFileSync(
        join(root, `${name}.test.ts`),
        `import {it,expect} from 'bun:test';import {${fn}} from './source';it('actual producer',()=>expect(${fn}()).toBe(${value}))`,
      )
    }
    const summary = await runTestGroups({
      root,
      config,
      groups: { isolated: ['a.test.ts'], runtime: ['b.test.ts'] },
      required: ['source.ts'],
    })
    expect(parseTestResult(summary)).toEqual({ pass: 2, fail: 0 })
    const source = parseLcov(
      readFileSync(join(root, 'coverage/lcov.info'), 'utf8'),
    ).find((record) => record.file === 'source.ts')
    expect(source?.foundFunctions).toBe(2)
    expect(source?.hitFunctions).toBe(2)
  } finally {
    output.mockRestore()
    rmSync(root, { recursive: true, force: true })
  }
}, 120000)

it.each([
  [
    'configuration',
    "import {writeFileSync} from 'node:fs';writeFileSync(process.env.DEVUP_PRODUCER_CONFIG,'[test]\\ncoverage=true\\ncoverageReporter=[\"lcov\"]\\ncoverageThreshold=0.1\\n')",
  ],
  [
    'new script',
    "process.stdin.on('data',chunk=>{if(String(chunk).includes('final evidence collected'))eval('globalThis.lateEvidence = 1')})",
  ],
] as const)(
  'refuses an actual %s change instead of accepting a stale final receipt',
  async (_name, change) => {
    const root = mkdtempSync(join(tmpdir(), 'devup-producer-change-'))
    try {
      const config = join(root, 'bunfig.toml')
      writeFileSync(
        config,
        '[test]\ncoverage=true\ncoverageReporter=["lcov"]\ncoverageThreshold=0.0\n',
      )
      writeFileSync(
        join(root, 'source.ts'),
        'export function value() { return 1 }\n',
      )
      writeFileSync(
        join(root, 'source.test.ts'),
        `import {it,expect} from 'bun:test';import {value} from './source';${change};it('actual',()=>expect(value()).toBe(1))`,
      )
      await expect(
        captureProcess({
          root,
          config,
          coverage: join(root, 'coverage'),
          group: 'isolated',
          files: ['source.test.ts'],
          executable: process.execPath,
        }),
      ).rejects.toThrow(CoverageError)
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  },
  120000,
)

it('refuses an existing native report before launching a producer that could reuse stale output', async () => {
  const root = mkdtempSync(join(tmpdir(), 'devup-producer-stale-'))
  try {
    const coverage = join(root, 'coverage')
    mkdirSync(coverage)
    writeFileSync(join(coverage, 'lcov.info'), 'stale native output')
    await expect(
      captureProcess({
        root,
        config: join(root, 'bunfig.toml'),
        coverage,
        group: 'isolated',
        files: ['source.test.ts'],
        executable: process.execPath,
      }),
    ).rejects.toThrow(CoverageError)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

it('refuses already parsed function execution when it occurs after the final raw observation', async () => {
  const root = mkdtempSync(join(tmpdir(), 'devup-producer-late-'))
  try {
    const config = join(root, 'bunfig.toml')
    writeFileSync(
      config,
      '[test]\npreload=["./late.ts"]\ncoverage=true\ncoverageReporter=["lcov"]\ncoverageThreshold=0.0\n',
    )
    writeFileSync(
      join(root, 'source.ts'),
      'export function value() {\n  return 1\n}\n',
    )
    writeFileSync(
      join(root, 'late.ts'),
      "import {value} from './source';process.stdin.on('data',chunk=>{if(String(chunk).includes('final evidence collected'))value()})\n",
    )
    writeFileSync(
      join(root, 'source.test.ts'),
      "import {it,expect} from 'bun:test';import {value} from './source';it('actual',()=>expect(value()).toBe(1))",
    )
    await expect(
      captureProcess({
        root,
        config,
        coverage: join(root, 'coverage'),
        group: 'isolated',
        files: ['source.test.ts'],
        executable: process.execPath,
      }),
    ).rejects.toThrow(CoverageError)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}, 120000)

it('binds the actual path-keyed native source when equivalent query imports reload one module', async () => {
  const root = mkdtempSync(join(tmpdir(), 'devup-producer-reload-'))
  try {
    const config = join(root, 'bunfig.toml')
    writeFileSync(
      config,
      '[test]\ncoverage=true\ncoverageReporter=["lcov"]\ncoverageThreshold=0.0\n',
    )
    writeFileSync(
      join(root, 'source.ts'),
      'export function value() { return 1 }\n',
    )
    writeFileSync(
      join(root, 'source.test.ts'),
      "import {it,expect} from 'bun:test';it('actual',async()=>{const first=await import('./source?first');expect(first.value()).toBe(1);const last=await import('./source?last');expect(last.value()).toBe(1)})",
    )
    const result = await captureProcess({
      root,
      config,
      coverage: join(root, 'coverage'),
      group: 'isolated',
      files: ['source.test.ts'],
      executable: process.execPath,
    })
    const bound = VerifiedSourceCoverage.bind(result, root)
    expect(
      bound.find((source) => source.file === 'source.ts')?.hitFunctions,
    ).toBe(1)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}, 120000)

it('retains early dynamically imported preload code when native coverage outlives its script object', async () => {
  const root = mkdtempSync(join(tmpdir(), 'devup-producer-early-'))
  try {
    // Given: source executes before the capture preload and is no longer retained by the importer.
    const config = join(root, 'bunfig.toml')
    writeFileSync(
      config,
      '[test]\npreload=["./early.ts"]\ncoverage=true\ncoverageReporter=["lcov"]\ncoverageThreshold=0.0\n',
    )
    writeFileSync(
      join(root, 'source.ts'),
      'export function first() { return 1 }\nfirst()\n',
    )
    writeFileSync(
      join(root, 'early.ts'),
      "await import('./source');Bun.gc(true)\n",
    )
    writeFileSync(
      join(root, 'source.test.ts'),
      "import {it,expect} from 'bun:test';it('actual',()=>expect(1).toBe(1))",
    )
    // When: the actual producer is bound to native coverage.
    const result = await captureProcess({
      root,
      config,
      coverage: join(root, 'coverage'),
      group: 'isolated',
      files: ['source.test.ts'],
      executable: process.execPath,
    })
    const bound = VerifiedSourceCoverage.bind(result, root)
    // Then: every early native source has its actual code, map and function receipt.
    expect(
      bound.find((source) => source.file === 'source.ts')?.hitFunctions,
    ).toBe(1)
    expect(bound.some((source) => source.file === 'early.ts')).toBe(true)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}, 120000)
