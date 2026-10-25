import { mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { auditCoverage } from '../../../../test-harness/coverage'
import { nativeFixture } from '../../../../test-harness/native-fixture'

it('covers complementary functions in one actual isolated native invocation', async () => {
  const fixture = nativeFixture({
    'source.ts':
      'export function first() {\n\n return 1\n}\nexport function second() {\n\n return 2\n}\n',
    'a.test.ts':
      "import {it,expect} from 'bun:test';import {first} from './source';it('first',()=>expect(first()).toBe(1))",
    'b.test.ts':
      "import {it,expect} from 'bun:test';import {second} from './source';it('second',()=>expect(second()).toBe(2))",
  })
  try {
    const result = await fixture.run(['a.test.ts', 'b.test.ts'])
    expect(result.code).toBe(0)
    expect(
      result.sources.find((source) => source.file === 'source.ts'),
    ).toMatchObject({ foundFunctions: 2, hitFunctions: 2 })
    expect(() => auditCoverage(result.sources, ['source.ts'])).not.toThrow()
    expect(result.identity).toMatchObject({
      executable: process.execPath,
      bun: Bun.version,
      revision: Bun.revision,
    })
  } finally {
    fixture.dispose()
  }
}, 120000)

it('refuses a pre-existing native report before launching', async () => {
  const fixture = nativeFixture({})
  try {
    mkdirSync(fixture.coverage)
    writeFileSync(join(fixture.coverage, 'lcov.info'), 'stale native output')
    await expect(fixture.run()).rejects.toThrow('stale native report')
  } finally {
    fixture.dispose()
  }
})

it('retains early dynamic preload execution after GC', async () => {
  const fixture = nativeFixture(
    {
      'source.ts':
        'export function first(){return 1}\nexport const observed=first()\n',
      'early.ts':
        "const source=await import('./source');globalThis.earlyResult=source.observed;Bun.gc(true)\n",
      'source.test.ts':
        "import {it,expect} from 'bun:test';it('early',()=>expect(globalThis.earlyResult).toBe(1))",
    },
    ['./early.ts'],
  )
  try {
    const result = await fixture.run()
    expect(result.code).toBe(0)
    expect(() =>
      auditCoverage(result.sources, ['source.ts', 'early.ts']),
    ).not.toThrow()
    expect(
      result.sources.find((source) => source.file === 'source.ts')
        ?.hitFunctions,
    ).toBe(1)
  } finally {
    fixture.dispose()
  }
}, 120000)
