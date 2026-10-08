import { spawn } from 'node:child_process'
import { randomUUID } from 'node:crypto'
import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import type { TestGroup } from './groups'
import { CoverageError } from './lcov'
import {
  object,
  parseBlocks,
  parseProducerIdentity,
  type ScriptCapture,
  text,
} from './producer-data'
import { connectProducer, type ProducerProtocol } from './producer-protocol'

export interface ProducerRun {
  readonly root: string
  readonly config: string
  readonly coverage: string
  readonly group: TestGroup
  readonly files: readonly string[]
  readonly executable: string
}
interface ProducerValue {
  readonly output: string
  readonly status: number | null
  readonly captures: readonly ScriptCapture[]
  readonly lcov: string
  readonly config: string
  readonly pid: number
  readonly bun: string
  readonly revision: string
}

const issuance = Symbol()
const issued = new WeakSet<object>()

export function isProducerResult(value: object): value is ProducerResult {
  return issued.has(value)
}

export class ProducerResult {
  readonly #value: ProducerValue

  private constructor(capability: typeof issuance, value: ProducerValue) {
    if (capability !== issuance)
      throw new CoverageError('unproven native producer issuance')
    this.#value = Object.freeze({
      ...value,
      captures: Object.freeze(value.captures),
    })
    Object.freeze(this)
    issued.add(this)
  }

  get value(): ProducerValue {
    if (!isProducerResult(this))
      throw new CoverageError('unproven native producer lifetime')
    return this.#value
  }

  static async capture(run: ProducerRun): Promise<ProducerResult> {
    if (existsSync(join(run.coverage, 'lcov.info')))
      throw new CoverageError('stale native producer output directory')
    const token = `DEVUP-PRODUCER-${randomUUID()}`
    const config = readFileSync(run.config, 'utf8')
    const preload = fileURLToPath(
      new URL('./producer-preload.ts', import.meta.url),
    )
    const child = spawn(
      run.executable,
      [
        '--inspect-wait=127.0.0.1:0/producer',
        `--config=${run.config}`,
        'test',
        `--preload=${preload}`,
        `--coverage-dir=${run.coverage}`,
        ...run.files.map((file) => `./${file}`),
      ],
      {
        cwd: run.root,
        stdio: ['pipe', 'pipe', 'pipe'],
        env: {
          ...process.env,
          DEVUP_TEST_GROUP: run.group,
          DEVUP_PRODUCER_TOKEN: token,
          DEVUP_PRODUCER_CONFIG: run.config,
        },
      },
    )
    let output = ''
    let protocol: ProducerProtocol | undefined
    let notify: (() => void) | undefined
    const records: Record<string, unknown>[] = []
    let remainder = ''
    const exit = new Promise<number | null>((done, reject) => {
      child.once('error', reject)
      child.once('exit', done)
    })
    const deadline = setTimeout(child.kill.bind(child), 600000)
    child.stdout.on('data', (chunk: Buffer) => {
      output += chunk.toString()
      remainder += chunk.toString()
      const lines = remainder.split('\n')
      remainder = lines.pop() ?? ''
      for (const line of lines) {
        if (line.startsWith(`${token} `))
          records.push(object(JSON.parse(line.slice(token.length + 1))))
      }
      notify?.()
    })
    child.stderr.on('data', (chunk: Buffer) => {
      output += chunk.toString()
      notify?.()
    })
    async function wait(condition: () => boolean): Promise<void> {
      if (condition()) return
      await new Promise<void>((done, reject) => {
        const timeout = setTimeout(
          reject.bind(
            undefined,
            new CoverageError('producer observation timeout'),
          ),
          600000,
        )
        notify = () => {
          if (condition()) {
            clearTimeout(timeout)
            done()
          }
        }
        exit.then(() => {
          clearTimeout(timeout)
          if (!condition())
            reject(
              new CoverageError(`producer exited before evidence: ${output}`),
            )
        }, reject)
      })
    }
    try {
      await wait(() => /ws:\/\/[^\s]+/.test(output))
      const url = /ws:\/\/[^\s]+/.exec(output)?.[0]
      if (!url) throw new CoverageError('missing producer inspector URL')
      protocol = await connectProducer(url)
      await protocol.request('Inspector.initialized')
      await wait(() =>
        records.some((record) => record['phase'] === 'bootstrap'),
      )
      const bootstrap = records.find(
        (record) => record['phase'] === 'bootstrap',
      )
      parseProducerIdentity(bootstrap, { pid: child.pid, config })
      child.stdin.write('bootstrap accepted\n')
      await wait(() => records.some((record) => record['phase'] === 'final'))
      const record = records.find((record) => record['phase'] === 'final')
      const { profiles, ...identity } = parseProducerIdentity(record, {
        pid: child.pid,
        config,
      })
      const captures: ScriptCapture[] = []
      let lateObserved = false
      const late = new Promise<void>((done) => {
        if (!protocol) throw new CoverageError('missing producer protocol')
        protocol.onEvent = (event) => {
          if (
            event['method'] === 'Debugger.paused' ||
            event['method'] === 'Debugger.scriptParsed'
          ) {
            lateObserved = true
            child.kill()
            done()
          }
        }
      })
      const scripts = protocol.events
        .filter((event) => event['method'] === 'Debugger.scriptParsed')
        .map((event) => object(event['params']))
      for (const event of scripts) {
        const scriptId = text(event['scriptId'])
        const path = text(event['url'])
        if (
          path.includes('node_modules') ||
          /(?:\.|_)(?:test|spec)\.(?:js|jsx|ts|tsx|mjs|cjs|mts|cts)$/.test(path)
        )
          continue
        if (!path || !existsSync(resolve(run.root, path.split('?')[0] ?? path)))
          continue
        const matching = profiles.filter(
          (profile) => profile.scriptId === scriptId,
        )
        if (matching.length > 1)
          throw new CoverageError('ambiguous producer precise identity')
        const source = object(
          await protocol.request('Debugger.getScriptSource', { scriptId }),
        )
        const blocks = object(
          await protocol.request('Runtime.getBasicBlocks', {
            sourceID: scriptId,
          }),
        )
        captures.push(
          Object.freeze({
            profile: matching[0],
            scriptId,
            url: resolve(run.root, path),
            scriptType: text(event['scriptType']),
            sourceMapURL: text(event['sourceMapURL']),
            code: text(source['scriptSource']),
            blocks: parseBlocks(blocks['basicBlocks']),
          }),
        )
      }
      await protocol.armEndFence(captures, preload)
      child.stdin.write('final evidence collected\n')
      await Promise.race([exit, late])
      if (lateObserved)
        throw new CoverageError(
          'late producer code/inventory after final capture',
        )
      const status = await exit
      if (
        records.filter((record) => record['phase'] === 'bootstrap').length !==
          1 ||
        records.filter((record) => record['phase'] === 'final').length !== 1
      )
        throw new CoverageError('ambiguous producer lifetime records')
      protocol.close()
      const lcov = readFileSync(join(run.coverage, 'lcov.info'), 'utf8')
      if (readFileSync(run.config, 'utf8') !== config)
        throw new CoverageError(
          'producer configuration changed during measurement',
        )
      const result = {
        ...identity,
        output: output
          .split('\n')
          .filter((line) => !line.startsWith(`${token} `))
          .join('\n'),
        status,
        captures,
        lcov,
      }
      writeFileSync(join(run.coverage, 'producer.json'), JSON.stringify(result))
      return new ProducerResult(issuance, result)
    } finally {
      clearTimeout(deadline)
      protocol?.close()
      if (child.exitCode === null) child.kill()
      await exit
    }
  }
}

export const captureProcess = ProducerResult.capture.bind(ProducerResult)
