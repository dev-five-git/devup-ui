import assert from 'node:assert/strict'
import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

import { nativeProcess } from '../../../../test-harness/native-process'
import { nativePublicationFixture } from '../../../../test-harness/native-publication-fixture'

it.each([false, true])(
  'invalidates old global acceptance on a real standalone Vite rerun failing=%s',
  async (failing) => {
    // Given a successful effective native script chain with real reports and acceptance.
    const fixture = nativePublicationFixture()
    const repository = resolve(import.meta.dir, '../../../..')
    const manifest: unknown = JSON.parse(
      readFileSync(join(repository, 'package.json'), 'utf8'),
    )
    assert(
      typeof manifest === 'object' &&
        manifest !== null &&
        'scripts' in manifest,
    )
    writeFileSync(
      join(fixture.root, 'package.json'),
      JSON.stringify({ type: 'module', scripts: manifest.scripts }),
    )
    const publication = JSON.stringify(
      join(repository, 'test-harness/publication.ts'),
    )
    writeFileSync(
      join(fixture.root, 'test-harness/publication-cli.ts'),
      `const p=await import(${publication});const root=process.cwd();switch(process.argv[2]){case 'begin':case 'standalone':p.beginPublication(root);break;case 'root':p.recordRoot(root,['source.ts']);break;case 'vite':p.recordVite(root,['source.ts']);break;case 'continue':p.continuePublication(root,['source.ts']);break;case 'accept':p.acceptPublication(root,{root:['source.ts'],vite:['source.ts']});break;default:throw new TypeError('Unknown fixture action')}`,
    )
    for (const [config, ignored] of [
      ['bunfig.toml', 'vite.test.ts'],
      ['bunfig.vite.toml', 'source.test.ts'],
    ] as const) {
      const path = join(fixture.root, config)
      writeFileSync(
        path,
        readFileSync(path, 'utf8') + `pathIgnorePatterns=["**/${ignored}"]\n`,
      )
    }
    const run = (script: string) =>
      nativeProcess({
        root: fixture.root,
        executable: process.execPath,
        args: ['run', script],
        timeout: 120000,
      })
    try {
      const first = await run('test:bun')
      expect(first.code, first.stdout + first.stderr).toBe(0)
      expect(existsSync(join(fixture.root, 'coverage/accepted.json'))).toBe(
        true,
      )
      if (failing)
        writeFileSync(
          join(fixture.root, 'vite.test.ts'),
          "import {it,expect} from 'bun:test';import {value} from './source';it('real failure',()=>expect(value()).toBe(0))",
        )
      // When the supported standalone Vite command reruns without a new ROOT lane.
      const second = await run('test:vite')
      console.info(
        JSON.stringify({
          event: 'native-standalone-rerun',
          failing,
          first,
          second,
        }),
      )
      // Then its actual outcome cannot preserve or create global acceptance.
      expect(second.code, second.stdout + second.stderr).toBe(failing ? 1 : 0)
      if (failing) expect(second.stderr).toContain('1 fail')
      expect(existsSync(join(fixture.root, 'coverage/accepted.json'))).toBe(
        false,
      )
      const continuation = await run('test:vite:continue')
      expect(continuation.code).not.toBe(0)
      expect(existsSync(join(fixture.root, 'coverage/accepted.json'))).toBe(
        false,
      )
    } finally {
      fixture.dispose()
    }
  },
  120000,
)
