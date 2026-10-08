import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it, spyOn } from 'bun:test'

import { CoverageError } from '../../../../test-harness/lcov'
import { runTestGroups } from '../../../../test-harness/run'

it('leaves no accepted artifact when actual loader outputs differ despite identical raw source', async () => {
  const root = mkdtempSync(join(tmpdir(), 'devup-coverage-publication-'))
  const output = spyOn(console, 'info').mockImplementation(() => undefined)
  const config = join(root, 'bunfig.toml')
  const accepted = join(root, 'coverage/lcov.info')
  try {
    mkdirSync(join(root, 'coverage'))
    writeFileSync(accepted, 'stale previously accepted report')
    writeFileSync(
      config,
      '[test]\npreload=["./transform.ts"]\ncoverage=true\ncoverageReporter=["lcov"]\ncoverageThreshold=0.0\n',
    )
    writeFileSync(
      join(root, 'source.ts'),
      'export function value() { return 1 }\n',
    )
    writeFileSync(
      join(root, 'transform.ts'),
      "import {plugin} from 'bun';await plugin({name:'actual-input',setup(build){build.onLoad({filter:/source\\.ts$/},()=>({contents:`export function value() { return ${process.env.DEVUP_TEST_GROUP==='runtime'?2:1} }\\n`,loader:'ts'}))}})",
    )
    for (const name of ['a', 'b']) {
      writeFileSync(
        join(root, `${name}.test.ts`),
        `import {it,expect} from 'bun:test';import {value} from './source';it('calls actual source',()=>expect(value()).toBe(${name === 'a' ? 1 : 2}))`,
      )
    }
    await expect(
      runTestGroups({
        root,
        config,
        groups: { isolated: ['a.test.ts'], runtime: ['b.test.ts'] },
        required: ['source.ts'],
      }),
    ).rejects.toThrow(CoverageError)
    expect(existsSync(accepted)).toBe(false)
  } finally {
    output.mockRestore()
    rmSync(root, { recursive: true, force: true })
  }
}, 120000)
