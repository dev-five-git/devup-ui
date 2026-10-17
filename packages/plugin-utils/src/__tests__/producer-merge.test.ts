import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it, spyOn } from 'bun:test'

import { parseLcov } from '../../../../test-harness/lcov'
import { runTestGroups } from '../../../../test-harness/run'

it('removes only proven coarse blank spans when real complementary functions cover the same mapped source', async () => {
  const root = mkdtempSync(join(tmpdir(), 'devup-producer-spans-'))
  const output = spyOn(console, 'info').mockImplementation(() => undefined)
  try {
    const config = join(root, 'bunfig.toml')
    writeFileSync(
      config,
      '[test]\ncoverage=true\ncoverageReporter=["lcov"]\ncoverageThreshold=0.0\ncoveragePathIgnorePatterns=["**/pkg/**","**/dist/**"]\n',
    )
    writeFileSync(
      join(root, 'source.ts'),
      'export function first() {\n\n  return 1\n}\nexport function second() {\n\n  return 2\n}\n',
    )
    for (const [name, fn, value] of [
      ['a', 'first', 1],
      ['b', 'second', 2],
    ] as const) {
      writeFileSync(
        join(root, `${name}.test.ts`),
        `import {it,expect} from 'bun:test';import {${fn}} from './source';it('actual',()=>expect(${fn}()).toBe(${value}))`,
      )
    }
    const summary = await runTestGroups({
      root,
      config,
      groups: { isolated: ['a.test.ts'], runtime: ['b.test.ts'] },
      required: ['source.ts'],
    })
    const source = parseLcov(
      readFileSync(join(root, 'coverage/lcov.info'), 'utf8'),
    ).find((item) => item.file === 'source.ts')
    expect(summary).toContain('2 pass\n0 fail')
    expect(source?.hitFunctions).toBe(2)
    expect(source?.foundFunctions).toBe(2)
    expect(source?.lines.has(2)).toBe(false)
    expect(source?.lines.has(6)).toBe(false)
    expect(source?.lines.get(3)).toBeGreaterThan(0)
    expect(source?.lines.get(7)).toBeGreaterThan(0)
  } finally {
    output.mockRestore()
    rmSync(root, { recursive: true, force: true })
  }
}, 120000)
