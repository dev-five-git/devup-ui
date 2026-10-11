import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it, spyOn } from 'bun:test'

import { auditCoverage } from '../../../../test-harness/coverage'
import { CoverageError } from '../../../../test-harness/lcov'
import { VerifiedSourceCoverage } from '../../../../test-harness/producer-coverage'
import { object, text } from '../../../../test-harness/producer-data'
import {
  captureProcess,
  ProducerResult,
} from '../../../../test-harness/producer-process'
import { ProducerProtocol } from '../../../../test-harness/producer-protocol'

function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'devup-producer-fixed-lifecycle-'))
  const config = join(root, 'bunfig.toml')
  writeFileSync(
    config,
    '[test]\ncoverage=true\ncoverageReporter=["lcov"]\ncoverageThreshold=0.0\ncoverageSkipTestFiles=true\n',
  )
  writeFileSync(
    join(root, 'source.ts'),
    'export function value() { return 42 }\n',
  )
  const files = ['handler.test.ts', 'peer.test.ts'] as const
  writeFileSync(
    join(root, files[0]),
    "import {writeSync} from 'node:fs';import {it,expect} from 'bun:test';import {value} from './source';process.on('SIGTERM',()=>{writeSync(1,'CLEANUP-HANDLER-47\\n');process.exit(47)});it('handler ready',()=>{writeSync(2,'HANDLER-READY\\n');expect(value()).toBe(42)})",
  )
  writeFileSync(
    join(root, files[1]),
    "import {it,expect} from 'bun:test';import {value} from './source';it('peer',()=>expect(value()).toBe(42))",
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

function closedFence(propagate: boolean) {
  let cause: CoverageError | undefined
  const original = ProducerProtocol.prototype.armEndFence
  const fence = spyOn(
    ProducerProtocol.prototype,
    'armEndFence',
  ).mockImplementation(async function (
    this: ProducerProtocol,
    captures,
    preload,
  ) {
    await original.call(this, captures, preload)
    const notification = new Promise<CoverageError>((resolve) => {
      const observe = this.onFailure
      this.onFailure = (error) => {
        observe?.(error)
        resolve(error)
      }
    })
    this.close()
    cause = await notification
    if (propagate) throw cause
  })
  return { fence, cause: () => cause }
}

function diagnostics(error: unknown, run: ReturnType<typeof fixture>) {
  if (!(error instanceof CoverageError)) throw error
  expect(error.cause).toBeInstanceOf(CoverageError)
  const context = object(object(error)['diagnostics'])
  expect(context['group']).toBe(run.group)
  expect(context['files']).toEqual(run.files)
  for (const file of run.files) expect(error.message).toContain(file)
  expect(Buffer.byteLength(text(context['stdoutTail']))).toBeLessThanOrEqual(
    8192,
  )
  expect(Buffer.byteLength(text(context['stderrTail']))).toBeLessThanOrEqual(
    8192,
  )
  expect(text(context['stderrTail'])).toContain('HANDLER-READY')
  return context
}

it('rejects the latched real socket failure when the original fence completes before final acknowledgement', async () => {
  // Given: an actual producer waits for its final acknowledgement after the original fence.
  const run = fixture()
  const wire = closedFence(false)
  try {
    // When: the real socket closes and delivers onFailure before the fence wrapper returns.
    const outcome = await captureProcess(run).then(
      (result) => result,
      (error: unknown) => error,
    )
    console.info(
      JSON.stringify({
        case: 'actual-post-fence-pre-ack',
        observed:
          outcome instanceof ProducerResult ? 'issued-receipt' : 'rejected',
        cause: String(wire.cause()),
      }),
    )
    // Then: no trusted receipt escapes and the contextual failure preserves the same cause.
    expect(outcome).toBeInstanceOf(CoverageError)
    if (!(outcome instanceof CoverageError)) throw outcome
    expect(outcome.cause).toBe(wire.cause())
    const context = diagnostics(outcome, run)
    console.info(JSON.stringify({ case: 'actual-latched-failure', context }))
  } finally {
    wire.fence.mockRestore()
    rmSync(run.root, { recursive: true, force: true })
  }
}, 120000)

it('retains only observed pre-cleanup origin when a real SIGTERM handler can change the final exit', async () => {
  // Given: the real child has installed a SIGTERM handler that exits47 with an output marker.
  const run = fixture()
  const wire = closedFence(true)
  try {
    // When: the real socket failure is propagated while the child still waits for final ack.
    const error = await captureProcess(run).then(
      () => null,
      (error: unknown) => error,
    )
    // Then: our cleanup cannot manufacture a natural status from the resulting exit code.
    if (!(error instanceof CoverageError)) throw error
    expect(error.cause).toBe(wire.cause())
    const context = diagnostics(error, run)
    const final = object(context['finalExit'])
    const stdout = text(context['stdoutTail'])
    expect(context['cleanupRequested']).toBe(true)
    expect(context['exitBeforeCleanup']).toBeNull()
    expect(context['naturalExit']).toBeNull()
    if (process.platform === 'win32' && final['code'] !== 47) {
      expect(final).toEqual({ code: null, signal: 'SIGTERM' })
      expect(stdout).not.toContain('CLEANUP-HANDLER-47')
    } else {
      expect(final).toEqual({ code: 47, signal: null })
      expect(stdout).toContain('CLEANUP-HANDLER-47')
    }
    console.info(
      JSON.stringify({
        case: 'actual-cleanup-handler',
        platform: process.platform,
        context,
      }),
    )
  } finally {
    wire.fence.mockRestore()
    rmSync(run.root, { recursive: true, force: true })
  }
}, 120000)

it('issues verified native coverage when the real child shuts down normally after final acknowledgement', async () => {
  // Given: the same real sources and handler, with the original inspector flow left intact.
  const run = fixture()
  try {
    // When: the ordinary final acknowledgement releases the producer.
    const result = await captureProcess(run)
    // Then: terminal socket shutdown still permits a genuine strictly covered receipt.
    expect(result.value.status).toBe(0)
    expect(() =>
      auditCoverage(VerifiedSourceCoverage.bind(result, run.root), [
        'source.ts',
      ]),
    ).not.toThrow()
    console.info(
      JSON.stringify({
        case: 'actual-normal-shutdown',
        bun: result.value.bun,
        pid: result.value.pid,
        status: result.value.status,
      }),
    )
  } finally {
    rmSync(run.root, { recursive: true, force: true })
  }
}, 120000)
