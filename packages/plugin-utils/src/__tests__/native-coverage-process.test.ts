import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { auditCoverage } from '../../../../test-harness/coverage'
import { nativeFixture } from '../../../../test-harness/native-fixture'
import {
  NativeLaunchError,
  nativeProcess,
} from '../../../../test-harness/native-process'

it.each(['process.exit(23)', 'process.abort()'])(
  'retains actual status and separated drained bounded tails on %s',
  async (termination) => {
    const fixture = nativeFixture(
      {
        'failure.ts': `import {writeSync} from 'node:fs';writeSync(1,'OUT-BEGIN'+'o'.repeat(10000)+'OUT-END\\n');writeSync(2,'ERR-BEGIN'+'e'.repeat(10000)+'ERR-END\\n');${termination}`,
        'source.test.ts':
          "import {it,expect} from 'bun:test';it('actual',()=>expect(1).toBe(1))",
      },
      ['./failure.ts'],
    )
    try {
      const result = await fixture.run()
      if (termination === 'process.exit(23)') expect(result.code).toBe(23)
      else expect(result.code !== 0 || result.signal !== null).toBe(true)
      expect(result.signal).not.toBe('SIGTERM')
      expect(result.drained).toBe(true)
      expect(result.cleanupRequested).toBe(false)
      expect(result.naturalExit).toEqual({
        code: result.code,
        signal: result.signal,
      })
      expect(result.stdout).toContain('OUT-BEGIN')
      expect(result.stderr).toContain('ERR-BEGIN')
      expect(result.stdoutTail).toContain('OUT-END')
      expect(result.stderrTail).toContain('ERR-END')
      expect(result.stdoutTail).not.toContain('ERR-END')
      expect(result.stderrTail).not.toContain('OUT-END')
      expect(result.stdoutTail).not.toContain('OUT-BEGIN')
      expect(result.stderrTail).not.toContain('ERR-BEGIN')
      expect(Buffer.byteLength(result.stdoutTail)).toBeLessThanOrEqual(8192)
      expect(Buffer.byteLength(result.stderrTail)).toBeLessThanOrEqual(8192)
    } finally {
      fixture.dispose()
    }
  },
  120000,
)

it.each([32, 550])(
  'retains all %s actually loaded and called native modules',
  async (count) => {
    const files = Array.from(
      { length: count },
      (_, index) => `module-${index}.ts`,
    )
    const fixture = nativeFixture({
      ...Object.fromEntries(
        files.map((file, index) => [
          file,
          `export function value(){return ${index}}\n`,
        ]),
      ),
      'source.test.ts': `import {it,expect} from 'bun:test';it('modules',async()=>{for(const [index,file] of ${JSON.stringify(files)}.entries()){const source=await import('./'+file);expect(source.value()).toBe(index)}})`,
    })
    try {
      const result = await fixture.run()
      expect(result.code).toBe(0)
      expect(
        result.sources.filter((source) => files.includes(source.file)),
      ).toHaveLength(count)
      expect(new Set(result.sources.map((source) => source.file)).size).toBe(
        result.sources.length,
      )
      expect(() => auditCoverage(result.sources, files)).not.toThrow()
    } finally {
      fixture.dispose()
    }
  },
  600000,
)

it('preserves the real OS missing-executable cause without waiting for a nonexistent exit', async () => {
  const fixture = nativeFixture({})
  try {
    const command = {
      root: fixture.root,
      executable: join(fixture.root, 'missing-executable'),
      args: ['first.test.ts', 'second.test.ts'],
      timeout: 5000,
    }
    const error = await nativeProcess(command).then(
      () => null,
      (error: unknown) => error,
    )
    expect(error).toBeInstanceOf(NativeLaunchError)
    if (!(error instanceof NativeLaunchError)) throw error
    expect(error.cause).toBeInstanceOf(Error)
    expect(String(error.cause)).toContain('missing-executable')
    expect(error.command).toBe(command)
    for (const file of command.args) expect(error.message).toContain(file)
  } finally {
    fixture.dispose()
  }
}, 120000)
