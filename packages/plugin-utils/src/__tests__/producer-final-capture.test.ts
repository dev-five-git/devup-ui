import { createHash } from 'node:crypto'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, it, jest, spyOn } from 'bun:test'

import { auditCoverage } from '../../../../test-harness/coverage'
import { CoverageError } from '../../../../test-harness/lcov'
import { ProducerChild } from '../../../../test-harness/producer-child'
import { VerifiedSourceCoverage } from '../../../../test-harness/producer-coverage'
import { object, text } from '../../../../test-harness/producer-data'
import {
  captureProcess,
  isProducerResult,
} from '../../../../test-harness/producer-process'
import { ProducerProtocol } from '../../../../test-harness/producer-protocol'
import { PRODUCER_TIMEOUT_MS } from '../../../../test-harness/producer-timeout'

function record(value: unknown) {
  console.info(JSON.stringify(value))
}

function observeChild() {
  const children = new Set<ProducerChild>()
  const original = ProducerChild.prototype.wait
  const spy = spyOn(ProducerChild.prototype, 'wait').mockImplementation(
    function (this: ProducerChild, condition) {
      children.add(this)
      return original.call(this, condition)
    },
  )
  return { spy, child: () => [...children][0] }
}

function fixture() {
  const root = mkdtempSync(join(tmpdir(), 'devup-producer-final-capture-'))
  const config = join(root, 'bunfig.toml')
  writeFileSync(
    config,
    '[test]\ncoverage=true\ncoverageReporter=["lcov"]\ncoverageThreshold=0.0\ncoverageSkipTestFiles=true\n',
  )
  writeFileSync(
    join(root, 'source.ts'),
    'export function value() { return 42 }\n',
  )
  const files = ['final.test.ts'] as const
  writeFileSync(
    join(root, files[0]),
    "import {writeSync} from 'node:fs';import {it,expect} from 'bun:test';import {value} from './source';it('real final capture',()=>{writeSync(1,'FINAL-OUT\\n');writeSync(2,'FINAL-ERR\\n');expect(value()).toBe(42)})",
  )
  return {
    root,
    config,
    coverage: join(root, 'coverage'),
    group: 'isolated' as const,
    files,
    executable: process.execPath,
  }
}

it.serial(
  'issues genuine native coverage when final acknowledgement is delayed beyond five seconds',
  async () => {
    // Given: a real child and source, with the original completed fence before a deliberate parent delay.
    const run = fixture()
    const observation = observeChild()
    let timer: ReturnType<typeof setTimeout> | undefined
    const original = ProducerProtocol.prototype.armEndFence
    let delay = 0
    const fence = spyOn(
      ProducerProtocol.prototype,
      'armEndFence',
    ).mockImplementation(async function (
      this: ProducerProtocol,
      captures,
      preload,
    ) {
      await original.call(this, captures, preload)
      const started = performance.now()
      await new Promise<void>((done) => {
        timer = setTimeout(done, 10000)
      })
      delay = performance.now() - started
    })
    try {
      // When: final stdin acknowledgement waits beyond the former hook default, without further RPC or evaluation.
      const result = await captureProcess(run)
      record({ case: 'delayed-final-ack', delay, status: result.value.status })
      // Then: the actual native producer survives and issues strictly authenticated coverage.
      expect(delay).toBeGreaterThan(5000)
      expect(result.value.status).toBe(0)
      expect(isProducerResult(result)).toBe(true)
      expect(Object.isFrozen(result)).toBe(true)
      expect(Object.isFrozen(result.value.captures)).toBe(true)
      const bound = VerifiedSourceCoverage.bind(result, run.root)
      expect(() => auditCoverage(bound, ['source.ts'])).not.toThrow()
      const capture = result.value.captures.find(
        (item) => item.url === join(run.root, 'source.ts'),
      )
      if (!capture) throw new CoverageError('missing final source witness')
      expect(capture.profile).toBeDefined()
      expect(capture.blocks.length).toBeGreaterThan(0)
      record({
        case: 'final-native-witness',
        bun: result.value.bun,
        revision: result.value.revision,
        pid: result.value.pid,
        config: result.value.config,
        sourceID: capture.scriptId,
        profile: capture.profile,
        blocks: capture.blocks,
        code: createHash('sha256').update(capture.code).digest('hex'),
        map: createHash('sha256').update(capture.sourceMapURL).digest('hex'),
        native: bound.map((source) => ({
          file: source.file,
          found: source.foundFunctions,
          hit: source.hitFunctions,
          lines: [...source.lines],
        })),
      })
    } finally {
      clearTimeout(timer)
      await observation.child()?.finish()
      fence.mockRestore()
      observation.spy.mockRestore()
      rmSync(run.root, { recursive: true, force: true })
    }
  },
  PRODUCER_TIMEOUT_MS,
)

it.serial(
  'registers the shared bound for a real producer and its pending evidence observations',
  async () => {
    // Given: the original timers and actual inspector flow, observed without replacing their behavior.
    const run = fixture()
    const timer = spyOn(globalThis, 'setTimeout')
    const delays: unknown[] = []
    const original = ProducerChild.prototype.wait
    const observation = spyOn(
      ProducerChild.prototype,
      'wait',
    ).mockImplementation(function (this: ProducerChild, condition) {
      const before = timer.mock.calls.length
      const pending = original.call(this, condition)
      delays.push(...timer.mock.calls.slice(before).map((call) => call[1]))
      return pending
    })
    try {
      // When: a real producer completes both lifetime captures.
      const result = await captureProcess(run)
      // Then: both deadline and registered observation timers use the existing shared bound.
      expect(result.value.status).toBe(0)
      expect(timer.mock.calls[0]?.[1]).toBe(PRODUCER_TIMEOUT_MS)
      expect(delays.length).toBeGreaterThan(0)
      expect(delays.every((delay) => delay === PRODUCER_TIMEOUT_MS)).toBe(true)
      record({
        case: 'registered-producer-bound',
        bun: result.value.bun,
        deadline: timer.mock.calls[0]?.[1],
        observations: delays,
      })
    } finally {
      timer.mockRestore()
      observation.mockRestore()
      rmSync(run.root, { recursive: true, force: true })
    }
  },
  PRODUCER_TIMEOUT_MS,
)

it.serial(
  'expires the real producer at the shared fake-timer boundary when final acknowledgement is withheld',
  async () => {
    // Given: a real child held after its original fence; only the parent's test-framework clock is virtual.
    const run = fixture()
    const ready = Promise.withResolvers<void>()
    const release = Promise.withResolvers<void>()
    const failed = Promise.withResolvers<CoverageError>()
    const observation = observeChild()
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
      const observe = this.onFailure
      this.onFailure = (error) => {
        observe?.(error)
        failed.resolve(error)
      }
      ready.resolve()
      await release.promise
    })
    jest.useFakeTimers()
    const timer = spyOn(globalThis, 'setTimeout')
    const pending = captureProcess(run).then(
      (result) => result,
      (error: unknown) => error,
    )
    try {
      await Promise.race([
        ready.promise,
        pending.then((outcome) => {
          throw outcome
        }),
      ])
      const producer = observation.child()
      if (!producer) throw new CoverageError('missing actual final producer')
      expect(timer.mock.calls[0]?.[1]).toBe(PRODUCER_TIMEOUT_MS)
      // When: the real unacknowledged child reaches its existing parent deadline under framework fake timers.
      jest.advanceTimersByTime(PRODUCER_TIMEOUT_MS - 1)
      expect(producer.child.exitCode).toBeNull()
      expect(producer.child.signalCode).toBeNull()
      expect(producer.child.killed).toBe(false)
      jest.advanceTimersByTime(1)
      expect(producer.child.killed).toBe(true)
      timer.mockRestore()
      jest.clearAllTimers()
      jest.useRealTimers()
      const cause = await failed.promise
      release.resolve()
      const outcome = await pending
      // Then: the original failure and drained, located actual exit survive; no trusted receipt is issued.
      expect(outcome).toBeInstanceOf(CoverageError)
      if (!(outcome instanceof CoverageError)) throw outcome
      expect(outcome.cause).toBe(cause)
      const context = object(object(outcome)['diagnostics'])
      expect(context['group']).toBe(run.group)
      expect(context['files']).toEqual(run.files)
      for (const file of run.files) expect(outcome.message).toContain(file)
      expect(context['cleanupRequested']).toBe(true)
      expect(context['naturalExit']).toBeNull()
      expect(context['exitBeforeCleanup']).toBeNull()
      const final = object(context['finalExit'])
      expect(final['code'] !== 0 || final['signal'] !== null).toBe(true)
      expect(
        producer.child.exitCode !== null || producer.child.signalCode !== null,
      ).toBe(true)
      for (const stream of ['stdoutTail', 'stderrTail']) {
        const bytes = Buffer.byteLength(text(context[stream]))
        expect(bytes).toBeGreaterThan(0)
        expect(bytes).toBeLessThanOrEqual(8192)
      }
      expect(text(context['stderrTail'])).toContain('FINAL-ERR')
      record({ case: 'fake-timer-final-expiry', bun: Bun.version, context })
    } finally {
      release.resolve()
      timer.mockRestore()
      if (jest.isFakeTimers()) jest.clearAllTimers()
      jest.useRealTimers()
      observation.child()?.terminate()
      await pending
      await observation.child()?.finish()
      fence.mockRestore()
      observation.spy.mockRestore()
      rmSync(run.root, { recursive: true, force: true })
    }
  },
  PRODUCER_TIMEOUT_MS,
)
