import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { CoverageError } from '../../../../test-harness/lcov'
import { parseProducerIdentity } from '../../../../test-harness/producer-data'
import { captureProcess } from '../../../../test-harness/producer-process'

it.each([
  "Object.defineProperty(Bun,'version',{value:'unsupported'})",
  "Object.defineProperty(Bun,'revision',{value:'unknown-revision'})",
  "Object.defineProperty(process,'pid',{value:0})",
] as const)(
  'refuses actual metadata tampering before bootstrap admission when %s',
  async (alteration) => {
    const root = mkdtempSync(join(tmpdir(), 'devup-producer-identity-'))
    try {
      const config = join(root, 'bunfig.toml')
      writeFileSync(
        config,
        '[test]\npreload=["./identity.ts"]\ncoverage=true\ncoverageReporter=["lcov"]\ncoverageThreshold=0.0\n',
      )
      writeFileSync(join(root, 'identity.ts'), alteration)
      writeFileSync(
        join(root, 'source.test.ts'),
        "import {it,expect} from 'bun:test';it('actual',()=>expect(1).toBe(1))",
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

it.each([
  { bun: 'unsupported' },
  { revision: 'unknown' },
  { pid: 0 },
  { config: 'stale' },
])(
  'rejects the altered identity $0 derived from an actual producer receipt',
  async (change) => {
    const root = mkdtempSync(
      join(tmpdir(), 'devup-producer-identity-boundary-'),
    )
    try {
      const config = join(root, 'bunfig.toml')
      writeFileSync(
        config,
        '[test]\ncoverage=true\ncoverageReporter=["lcov"]\ncoverageThreshold=0.0\n',
      )
      writeFileSync(
        join(root, 'source.test.ts'),
        "import {it,expect} from 'bun:test';it('actual',()=>expect(1).toBe(1))",
      )
      const result = await captureProcess({
        root,
        config,
        coverage: join(root, 'coverage'),
        group: 'isolated',
        files: ['source.test.ts'],
        executable: process.execPath,
      })
      const expected = { pid: result.value.pid, config: result.value.config }
      expect(() =>
        parseProducerIdentity({ ...result.value, ...change }, expected),
      ).toThrow(CoverageError)
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  },
  120000,
)

it('rejects duplicate final handshakes when actual test code emits an extra token-bearing record', async () => {
  const root = mkdtempSync(join(tmpdir(), 'devup-producer-duplicate-'))
  try {
    const config = join(root, 'bunfig.toml')
    writeFileSync(
      config,
      '[test]\ncoverage=true\ncoverageReporter=["lcov"]\ncoverageThreshold=0.0\n',
    )
    writeFileSync(
      join(root, 'source.test.ts'),
      "import {it,expect} from 'bun:test';const original=process.stdout.write.bind(process.stdout);process.stdout.write=(chunk,...args)=>{const text=String(chunk);if(text.includes('\\\"phase\\\":\\\"final\\\"'))original(chunk,...args);return original(chunk,...args)};it('actual',()=>expect(1).toBe(1))",
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
