import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { CoverageError } from '../../../../test-harness/lcov'
import { VerifiedSourceCoverage } from '../../../../test-harness/producer-coverage'
import { parseProducerIdentity } from '../../../../test-harness/producer-data'
import { captureProcess } from '../../../../test-harness/producer-process'

const otherIdentity =
  Bun.version === '1.4.0'
    ? { bun: '1.4.2', revision: '744846f844374847c902b5e7fd59b4342a51ef99' }
    : { bun: '1.4.0', revision: '34cbb9a40b4bd1bd767d134a7065e66c2432a676' }

it('admits the actual qualified runtime only when its native coverage can be bound', async () => {
  // Given: the real current executable, not a fabricated accepted identity.
  const root = mkdtempSync(join(tmpdir(), 'devup-producer-qualified-'))
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
      "import {it,expect} from 'bun:test';import {value} from './source';it('actual',()=>expect(value()).toBe(1))",
    )
    // When: the actual producer issues a receipt and native replay binds it.
    const result = await captureProcess({
      root,
      config,
      coverage: join(root, 'coverage'),
      group: 'isolated',
      files: ['source.test.ts'],
      executable: process.execPath,
    })
    const source = VerifiedSourceCoverage.bind(result, root).find(
      (source) => source.file === 'source.ts',
    )
    // Then: both runtime identity and its executed native function are retained.
    expect([result.value.bun, result.value.revision]).toEqual([
      Bun.version,
      Bun.revision,
    ])
    expect([source?.foundFunctions, source?.hitFunctions]).toEqual([1, 1])
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}, 120000)

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
  { bun: otherIdentity.bun },
  { revision: otherIdentity.revision },
  otherIdentity,
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
      const validate = () =>
        parseProducerIdentity({ ...result.value, ...change }, expected)
      expect(validate).toThrow(CoverageError)
      expect(validate).toThrow(
        'unknown/stale producer process or configuration identity',
      )
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  },
  120000,
)

it.each(['bootstrap', 'final'] as const)(
  'refuses another qualified version/revision when actual code substitutes it at %s',
  async (phase) => {
    // Given: substituting a valid pair differs from an unknown or crossed pair.
    const root = mkdtempSync(join(tmpdir(), 'devup-producer-mixed-version-'))
    const alteration = `const original=process.stdout.write.bind(process.stdout);process.stdout.write=(chunk,...args)=>{const text=String(chunk);if(text.startsWith(process.env.DEVUP_PRODUCER_TOKEN+' ')&&text.includes(${JSON.stringify(`"phase":"${phase}"`)})){const separator=text.indexOf(' ');const record=JSON.parse(text.slice(separator+1));return original(text.slice(0,separator+1)+JSON.stringify({...record,...${JSON.stringify(otherIdentity)}})+'\\n',...args)}return original(chunk,...args)}`
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
      // When / Then: admission refuses before another lifetime can be issued.
      await expect(
        captureProcess({
          root,
          config,
          coverage: join(root, 'coverage'),
          group: 'isolated',
          files: ['source.test.ts'],
          executable: process.execPath,
        }),
      ).rejects.toThrow(
        'unknown/stale producer process or configuration identity',
      )
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
