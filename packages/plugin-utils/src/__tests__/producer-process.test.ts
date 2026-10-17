import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it, spyOn } from 'bun:test'

import { auditCoverage } from '../../../../test-harness/coverage'
import { CoverageError, parseLcov } from '../../../../test-harness/lcov'
import { VerifiedSourceCoverage } from '../../../../test-harness/producer-coverage'
import { object, text } from '../../../../test-harness/producer-data'
import { captureProcess } from '../../../../test-harness/producer-process'
import { ProducerProtocol } from '../../../../test-harness/producer-protocol'

function fixture(termination: string) {
  const root = mkdtempSync(join(tmpdir(), 'devup-producer-failure-'))
  const config = join(root, 'bunfig.toml')
  writeFileSync(
    config,
    '[test]\npreload=["./failure.ts"]\ncoverage=true\ncoverageReporter=["lcov"]\ncoverageThreshold=0.0\n',
  )
  writeFileSync(
    join(root, 'failure.ts'),
    `import {writeSync} from 'node:fs';writeSync(1,'OUT-BEGIN'+'o'.repeat(10000)+'OUT-END\\n');writeSync(2,'ERR-BEGIN'+'e'.repeat(10000)+'ERR-END\\n');${termination}`,
  )
  const files = ['first.test.ts', 'second.test.ts']
  for (const file of files)
    writeFileSync(
      join(root, file),
      "import {it,expect} from 'bun:test';it('actual',()=>expect(1).toBe(1))",
    )
  return {
    root,
    config,
    coverage: join(root, 'coverage'),
    group: 'runtime' as const,
    files,
    executable: process.execPath,
  }
}

function diagnostics(error: unknown, run: ReturnType<typeof fixture>) {
  if (!(error instanceof CoverageError)) throw error
  expect(error.cause).toBeInstanceOf(Error)
  const context = object(object(error)['diagnostics'])
  expect(context['group']).toBe(run.group)
  expect(context['files']).toEqual(run.files)
  for (const file of run.files) expect(error.message).toContain(file)
  const stdout = text(context['stdoutTail'])
  const stderr = text(context['stderrTail'])
  expect(Buffer.byteLength(stdout)).toBeLessThanOrEqual(8192)
  expect(Buffer.byteLength(stderr)).toBeLessThanOrEqual(8192)
  expect(stdout).not.toContain('ERR-END')
  expect(stderr).not.toContain('OUT-END')
  expect(stdout).not.toContain('OUT-BEGIN')
  expect(stderr).not.toContain('ERR-BEGIN')
  console.info(JSON.stringify({ case: 'actual-producer-failure', context }))
  return context
}

it.each([
  ['early exit', 'process.exit(23)', 23],
  ['abrupt crash', 'process.abort()', null],
] as const)(
  'retains actual final status and both drained tails when a real producer suffers %s before completion',
  async (_name, termination, code) => {
    // Given: two actual test files and a real preload that exits/crashes.
    const run = fixture(termination)
    try {
      // When: the actual producer fails without issuing final evidence.
      const error = await captureProcess(run).then(
        () => null,
        (error: unknown) => error,
      )
      // Then: actual final status survives, with natural origin limited to pre-cleanup observation.
      const context = diagnostics(error, run)
      expect(text(context['stdoutTail'])).toContain('OUT-END')
      expect(text(context['stderrTail'])).toContain('ERR-END')
      const final = object(context['finalExit'])
      if (code === 23) expect(final).toEqual({ code: 23, signal: null })
      else {
        expect(final['code'] !== 0 || final['signal'] !== null).toBe(true)
        expect(final['signal']).not.toBe('SIGTERM')
      }
      expect(context['naturalExit']).toEqual(context['exitBeforeCleanup'])
    } finally {
      rmSync(run.root, { recursive: true, force: true })
    }
  },
  120000,
)

it('preserves the socket cause and labels cleanup separately when the real child is still alive', async () => {
  // Given: the real inspector closes during collection while the child waits.
  const run = fixture('')
  let cause: unknown
  const original = ProducerProtocol.prototype.request
  const request = spyOn(
    ProducerProtocol.prototype,
    'request',
  ).mockImplementation(async function (
    this: ProducerProtocol,
    method,
    params = {},
  ) {
    if (method === 'Debugger.getScriptSource') {
      this.close()
      try {
        return await original.call(this, method, params)
      } catch (error) {
        cause = error
        throw error
      }
    }
    return original.call(this, method, params)
  })
  try {
    // When: a real socket closes before the final capture completes.
    const error = await captureProcess(run).then(
      () => null,
      (error: unknown) => error,
    )
    // Then: own cleanup is never represented as a natural crash.
    if (!(error instanceof CoverageError)) throw error
    expect(error.cause).toBe(cause)
    const context = diagnostics(error, run)
    expect(context['naturalExit']).toBeNull()
    expect(context['cleanupRequested']).toBe(true)
    expect(object(context['finalExit'])).toHaveProperty('signal')
  } finally {
    request.mockRestore()
    rmSync(run.root, { recursive: true, force: true })
  }
}, 120000)

it('rejects a real socket close when the child is alive with no pending RPC', async () => {
  // Given: a real child and inspector, with bootstrap observation still pending.
  const run = fixture('')
  const original = ProducerProtocol.prototype.request
  const request = spyOn(
    ProducerProtocol.prototype,
    'request',
  ).mockImplementation(async function (
    this: ProducerProtocol,
    method,
    params = {},
  ) {
    const result = await original.call(this, method, params)
    if (method === 'Inspector.initialized') this.close()
    return result
  })
  try {
    // When: the real socket closes outside an active request.
    const error = await captureProcess(run).then(
      () => null,
      (error: unknown) => error,
    )
    // Then: failure rejects promptly with its wire cause instead of waiting for timeout.
    if (!(error instanceof CoverageError)) throw error
    expect(error.cause).toBeInstanceOf(CoverageError)
    expect(String(error.cause)).toContain('producer socket closed')
    expect(object(object(error)['diagnostics'])['files']).toEqual(run.files)
  } finally {
    request.mockRestore()
    rmSync(run.root, { recursive: true, force: true })
  }
}, 5000)

it.each([32, 550])(
  'retains every real native module when the producer loads %s unique sources',
  async (count) => {
    // Given: unique real source files, all loaded and called by the actual child.
    const root = mkdtempSync(join(tmpdir(), 'devup-producer-scale-'))
    try {
      const config = join(root, 'bunfig.toml')
      writeFileSync(
        config,
        '[test]\ncoverage=true\ncoverageReporter=["lcov"]\ncoverageThreshold=0.0\n',
      )
      const files = Array.from(
        { length: count },
        (_, index) => `module-${index}.ts`,
      )
      for (const [index, file] of files.entries())
        writeFileSync(
          join(root, file),
          `export function value() { return ${index} }\n`,
        )
      writeFileSync(
        join(root, 'scale.test.ts'),
        `import {it,expect} from 'bun:test';it('actual modules',async()=>{for(const [index,file] of ${JSON.stringify(files)}.entries()){const source=await import('./'+file);expect(source.value()).toBe(index)}})`,
      )
      // When: the real producer/protocol captures code, maps, blocks and native rows.
      const result = await captureProcess({
        root,
        config,
        coverage: join(root, 'coverage'),
        group: 'isolated',
        files: ['scale.test.ts'],
        executable: process.execPath,
      })
      const bound = VerifiedSourceCoverage.bind(result, root)
      // Then: the unchanged auditor requires every original source at strict100.
      expect(result.value.status).toBe(0)
      expect(() => auditCoverage(bound, files)).not.toThrow()
      expect(
        parseLcov(result.value.lcov).filter((source) =>
          files.includes(source.file),
        ).length,
      ).toBe(count)
      const witnesses = files.map((file) => {
        const capture = result.value.captures.find(
          (capture) => capture.url === join(root, file),
        )
        if (!capture)
          throw new CoverageError(`missing actual scale witness ${file}`)
        return {
          file,
          sourceID: capture.scriptId,
          blocks: capture.blocks.length,
          code: createHash('sha256').update(capture.code).digest('hex'),
          map: createHash('sha256').update(capture.sourceMapURL).digest('hex'),
        }
      })
      expect(new Set(witnesses.map((witness) => witness.sourceID)).size).toBe(
        count,
      )
      console.info(
        JSON.stringify({
          case: 'actual-producer-scale',
          count,
          bun: result.value.bun,
          revision: result.value.revision,
          pid: result.value.pid,
          nativeSources: bound.length,
          witnesses,
        }),
      )
    } finally {
      rmSync(root, { recursive: true, force: true })
    }
  },
  600000,
)

it('retains the original spawn error without waiting for a nonexistent exit event when the executable is missing', async () => {
  // Given: a real spawn of a nonexistent executable.
  const run = fixture('')
  try {
    // When: the operating system refuses the launch.
    const error = await captureProcess({
      ...run,
      executable: join(run.root, 'missing-executable'),
    }).then(
      () => null,
      (error: unknown) => error,
    )
    // Then: contextual failure retains the actual OS cause and every file.
    if (!(error instanceof CoverageError)) throw error
    expect(error.cause).toBeInstanceOf(Error)
    expect(String(error.cause)).toContain('missing-executable')
    const context = object(object(error)['diagnostics'])
    expect(context['files']).toEqual(run.files)
    expect(context['naturalExit']).toBeNull()
    expect(context['cleanupRequested']).toBe(false)
  } finally {
    rmSync(run.root, { recursive: true, force: true })
  }
}, 120000)
import { createHash } from 'node:crypto'
