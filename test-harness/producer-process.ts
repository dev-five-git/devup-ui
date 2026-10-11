import { randomUUID } from 'node:crypto'
import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { isAbsolute, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import type { TestGroup } from './groups'
import { CoverageError } from './lcov'
import { ProducerChild } from './producer-child'
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
    const producer = new ProducerChild(run, { token, preload })
    const { child, exit } = producer
    let protocol: ProducerProtocol | undefined
    const records: Record<string, unknown>[] = []
    let remainder = ''
    child.stdout.prependListener('data', (chunk: Buffer) => {
      remainder += chunk.toString()
      const lines = remainder.split('\n')
      remainder = lines.pop() ?? ''
      for (const line of lines) {
        if (line.startsWith(`${token} `))
          records.push(object(JSON.parse(line.slice(token.length + 1))))
      }
    })
    try {
      await producer.wait(() => /ws:\/\/[^\s]+/.test(producer.output))
      const url = /ws:\/\/[^\s]+/.exec(producer.output)?.[0]
      if (!url) throw new CoverageError('missing producer inspector URL')
      protocol = await connectProducer(url)
      protocol.onFailure = producer.observeFailure.bind(producer)
      await protocol.request('Inspector.initialized')
      await producer.wait(() =>
        records.some((record) => record['phase'] === 'bootstrap'),
      )
      const bootstrap = records.find(
        (record) => record['phase'] === 'bootstrap',
      )
      parseProducerIdentity(bootstrap, { pid: child.pid, config })
      child.stdin.write('bootstrap accepted\n')
      await producer.wait(() =>
        records.some((record) => record['phase'] === 'final'),
      )
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
            producer.terminate()
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
        const file = resolve(run.root, path.split('?')[0] ?? path)
        const mappedFile = isAbsolute(path) && Boolean(event['sourceMapURL'])
        if (!path || (!mappedFile && !existsSync(file))) continue
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
      producer.assertObservation()
      child.stdin.write('final evidence collected\n')
      await Promise.race([exit, late])
      if (lateObserved)
        throw new CoverageError(
          'late producer code/inventory after final capture',
        )
      const status = await exit
      await producer.finish()
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
        output: producer.output
          .split('\n')
          .filter((line) => !line.startsWith(`${token} `))
          .join('\n'),
        status,
        captures,
        lcov,
      }
      writeFileSync(join(run.coverage, 'producer.json'), JSON.stringify(result))
      return new ProducerResult(issuance, result)
    } catch (error) {
      protocol?.close()
      await producer.finish()
      if (!(error instanceof Error)) throw error
      throw producer.failure(error)
    } finally {
      protocol?.close()
    }
  }
}

export const captureProcess = ProducerResult.capture.bind(ProducerResult)
