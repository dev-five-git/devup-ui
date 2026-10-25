import { mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { nativeFixture } from './native-fixture'
import { nativeProcess } from './native-process'
import type { NativeLane } from './publication'

export function nativePublicationFixture(second = 'expect(value()).toBe(42)') {
  const fixture = nativeFixture({
    'source.ts': 'export function value(){return 42}\n',
    'source.test.ts':
      "import {it,expect} from 'bun:test';import {value} from './source';it('root',()=>expect(value()).toBe(42))",
    'vite.test.ts': `import {it,expect} from 'bun:test';import {value} from './source';it('vite',()=>{${second}})`,
  })
  for (const directory of ['packages', 'test-harness'])
    mkdirSync(join(fixture.root, directory))
  for (const file of ['bun.setup.ts', 'bun.runtime.ts', 'bun.matrix.ts'])
    writeFileSync(join(fixture.root, file), 'export {}\n')
  writeFileSync(
    join(fixture.root, 'package.json'),
    '{"scripts":{"test":"native root then vite"}}',
  )
  for (const [lane, name] of [
    ['root', 'bunfig.toml'],
    ['vite', 'bunfig.vite.toml'],
  ] as const) {
    writeFileSync(
      join(fixture.root, name),
      `[test]\ncoverageReporter=["text","lcov"]\ncoverageThreshold={lines=1.0,functions=1.0,statements=1.0}\ncoverageSkipTestFiles=true\ncoverageDir="coverage/${lane}"\n`,
    )
  }
  return {
    ...fixture,
    async lane(lane: NativeLane) {
      const config = join(
        fixture.root,
        lane === 'root' ? 'bunfig.toml' : 'bunfig.vite.toml',
      )
      return nativeProcess({
        root: fixture.root,
        executable: process.execPath,
        args: [
          `--config=${config}`,
          'test',
          '--coverage',
          ...(lane === 'root' ? ['--isolate'] : []),
          lane === 'root' ? './source.test.ts' : './vite.test.ts',
        ],
        timeout: 120000,
      })
    },
  }
}
