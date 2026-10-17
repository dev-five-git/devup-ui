import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { setTimeout as delay } from 'node:timers/promises'

import { expect, it, spyOn } from 'bun:test'

import { VerifiedSourceCoverage } from '../../../../test-harness/producer-coverage'
import { array, object } from '../../../../test-harness/producer-data'
import {
  captureProcess,
  isProducerResult,
} from '../../../../test-harness/producer-process'
import { ProducerProtocol } from '../../../../test-harness/producer-protocol'
import { parseTestResult, rootTestRun } from '../../../../test-harness/run'

it.serial(
  'disposes watch observation deadlines before final native capture',
  async () => {
    // Given the three original real-watch cases and a pass-through native timer observer.
    const root = resolve(import.meta.dir, '../../../..')
    const run = rootTestRun(root)
    const files = [
      'packages/vite-plugin/src/__tests__/resolution-watch-environments.test.ts',
      'packages/vite-plugin/src/__tests__/resolution-watch.test.ts',
    ]
    const fixture = mkdtempSync(join(tmpdir(), 'devup-watch-deadlines-'))
    const journal = join(fixture, 'timers.jsonl')
    const observer = join(fixture, 'observer.test.mjs')
    const config = join(fixture, 'bunfig.toml')
    const original = ProducerProtocol.prototype.armEndFence
    let hold = 0
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
      await delay(11000)
      hold = performance.now() - started
    })
    try {
      writeFileSync(
        observer,
        `import {openSync,writeSync,closeSync} from 'node:fs'
const fd = openSync(${JSON.stringify(journal)}, 'w')
const owners = ${JSON.stringify(files.map((file) => join(root, file).replaceAll('\\', '/')))}
const pending = new Map()
const timeout = globalThis.setTimeout
const clear = globalThis.clearTimeout
const write = process.stdout.write
let final = false
let sequence = 0
const emit = (record) => writeSync(fd, JSON.stringify(record) + '\\n')
const observedTimeout = new Proxy(timeout, {
  apply(target, receiver, args) {
    const [callback, duration, ...rest] = args
    const stack = new Error('watch deadline registration').stack.replaceAll('\\\\', '/')
    const owner = owners.find((file) => stack.includes(file + ':'))
    if (!owner || typeof callback !== 'function') return Reflect.apply(target, receiver, args)
    const record = {id: ++sequence, owner, duration, stack}
    let timer
    timer = Reflect.apply(target, receiver, [function (...values) {
      pending.delete(timer)
      emit({action:'fire', afterFinal:final, ...record})
      return Reflect.apply(callback, this, values)
    }, duration, ...rest])
    pending.set(timer, record)
    emit({action:'register', ...record})
    return timer
  },
})
const observedClear = new Proxy(clear, {
  apply(target, receiver, args) {
    const record = pending.get(args[0])
    if (record) { pending.delete(args[0]); emit({action:'clear', ...record}) }
    return Reflect.apply(target, receiver, args)
  },
})
globalThis.setTimeout = observedTimeout
globalThis.clearTimeout = observedClear
process.stdout.write = new Proxy(write, {
  apply(target, receiver, args) {
    const chunk = String(args[0])
    if (chunk.startsWith(process.env.DEVUP_PRODUCER_TOKEN + ' ') && chunk.includes('"phase":"final"')) {
      final = true
      emit({action:'final', pending:[...pending.values()]})
    }
    return Reflect.apply(target, receiver, args)
  },
})
process.once('exit', () => {
  if (globalThis.setTimeout === observedTimeout) globalThis.setTimeout = timeout
  if (globalThis.clearTimeout === observedClear) globalThis.clearTimeout = clear
  process.stdout.write = write
  closeSync(fd)
})
`,
      )
      const require = createRequire(join(root, 'package.json'))
      const preloads = [
        observer,
        join(root, 'bun.preload.ts'),
        join(root, 'bun.setup.ts'),
        require.resolve('bun-test-env-dom'),
      ].map((file) => file.replaceAll('\\', '/'))
      writeFileSync(
        config,
        readFileSync(run.config, 'utf8')
          .replace(
            /^root = .*$/m,
            `root = ${JSON.stringify(join(root, 'packages').replaceAll('\\', '/'))}`,
          )
          .replace(/^preload = .*$/m, `preload = ${JSON.stringify(preloads)}`),
      )
      // When the original fence completes, leave the real child awaiting ack beyond its deadlines.
      const started = performance.now()
      const result = await captureProcess({
        root,
        config,
        coverage: join(fixture, 'coverage'),
        group: 'runtime',
        files,
        executable: process.execPath,
      })
      const records = readFileSync(journal, 'utf8')
        .trim()
        .split('\n')
        .map((line) => object(JSON.parse(line)))
      const snapshots = records.filter((record) => record['action'] === 'final')
      const snapshot = object(snapshots[0])
      const late = records.filter(
        (record) =>
          record['action'] === 'fire' && record['afterFinal'] === true,
      )
      const bound = VerifiedSourceCoverage.bind(result, root)
      console.info(
        JSON.stringify({
          case: 'watch-deadline-cleanup',
          bun: result.value.bun,
          revision: result.value.revision,
          pid: result.value.pid,
          elapsed: performance.now() - started,
          hold,
          records,
          sources: bound.length,
          status: result.value.status,
        }),
      )
      // Then all original blue-CSS cases pass with genuine native evidence and zero surviving deadlines.
      expect(result.value.status).toBe(0)
      expect(isProducerResult(result)).toBe(true)
      expect(parseTestResult(result.value.output)).toEqual({ pass: 3, fail: 0 })
      expect(hold).toBeGreaterThanOrEqual(11000)
      expect(snapshots).toHaveLength(1)
      for (const file of files) {
        expect(
          records.some(
            (record) =>
              record['action'] === 'register' &&
              record['owner'] === join(root, file).replaceAll('\\', '/'),
          ),
        ).toBe(true)
      }
      expect(
        bound.some(
          (source) =>
            source.file === 'packages/vite-plugin/src/resolution-watch.ts' &&
            source.hitFunctions > 0,
        ),
      ).toBe(true)
      expect(
        result.value.captures.some((capture) => capture.url === observer),
      ).toBe(false)
      expect({ pending: array(snapshot['pending']), late }).toEqual({
        pending: [],
        late: [],
      })
    } finally {
      fence.mockRestore()
      rmSync(fixture, { recursive: true, force: true })
    }
  },
  120000,
)
