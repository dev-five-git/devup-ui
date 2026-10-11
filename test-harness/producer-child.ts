import { type ChildProcessWithoutNullStreams, spawn } from 'node:child_process'
import { setImmediate } from 'node:timers/promises'

import type { TestGroup } from './groups'
import { CoverageError } from './lcov'
import type { ProducerRun } from './producer-process'
import { PRODUCER_TIMEOUT_MS } from './producer-timeout'

interface ExitStatus {
  readonly code: number | null
  readonly signal: NodeJS.Signals | null
}

interface ProducerDiagnostics {
  readonly group: TestGroup
  readonly files: readonly string[]
  readonly naturalExit: ExitStatus | null
  readonly exitBeforeCleanup: ExitStatus | null
  readonly cleanupRequested: boolean
  readonly finalExit: ExitStatus | null
  readonly stdoutTail: string
  readonly stderrTail: string
}

class ProducerFailure extends CoverageError {
  constructor(
    override readonly cause: Error,
    readonly diagnostics: ProducerDiagnostics,
  ) {
    super(`${cause.message}\nproducer failure: ${JSON.stringify(diagnostics)}`)
  }
}

export class ProducerChild {
  readonly child: ChildProcessWithoutNullStreams
  readonly exit: Promise<number | null>
  private readonly deadline: ReturnType<typeof setTimeout>
  private stdoutTail = Buffer.alloc(0)
  private stderrTail = Buffer.alloc(0)
  private naturalExit: ExitStatus | null = null
  private exitBeforeCleanup: ExitStatus | null = null
  private finalExit: ExitStatus | null = null
  private spawnError: Error | undefined
  private observationError: CoverageError | undefined
  private cleanupRequested = false
  private notify: (() => void) | undefined
  output = ''

  constructor(
    private readonly run: ProducerRun,
    bootstrap: { readonly token: string; readonly preload: string },
  ) {
    try {
      this.child = spawn(
        run.executable,
        [
          '--inspect-wait=127.0.0.1:0/producer',
          `--config=${run.config}`,
          'test',
          `--preload=${bootstrap.preload}`,
          `--coverage-dir=${run.coverage}`,
          ...run.files.map((file) => `./${file}`),
        ],
        {
          cwd: run.root,
          stdio: ['pipe', 'pipe', 'pipe'],
          env: {
            ...process.env,
            DEVUP_TEST_GROUP: run.group,
            DEVUP_PRODUCER_TOKEN: bootstrap.token,
            DEVUP_PRODUCER_CONFIG: run.config,
          },
        },
      )
    } catch (error) {
      if (!(error instanceof Error)) throw error
      this.recordSpawnError(error)
      throw new ProducerFailure(error, this.diagnostics())
    }
    this.child.once('error', this.recordSpawnError.bind(this))
    this.child.once('exit', (code, signal) => {
      this.finalExit = { code, signal }
      if (!this.cleanupRequested)
        this.exitBeforeCleanup = this.naturalExit = this.finalExit
    })
    this.exit = new Promise((done) => this.child.once('close', done))
    this.deadline = setTimeout(this.terminate.bind(this), PRODUCER_TIMEOUT_MS)
    this.child.stdout.on('data', (chunk: Buffer) => {
      this.output += chunk.toString()
      this.stdoutTail = Buffer.from(
        Buffer.concat([this.stdoutTail, chunk]).subarray(-8192),
      )
      this.notify?.()
    })
    this.child.stderr.on('data', (chunk: Buffer) => {
      this.output += chunk.toString()
      this.stderrTail = Buffer.from(
        Buffer.concat([this.stderrTail, chunk]).subarray(-8192),
      )
      this.notify?.()
    })
  }

  async wait(condition: () => boolean): Promise<void> {
    this.assertObservation()
    if (condition()) return
    await new Promise<void>((done, reject) => {
      const timeout = setTimeout(
        reject.bind(
          undefined,
          new CoverageError('producer observation timeout'),
        ),
        PRODUCER_TIMEOUT_MS,
      )
      this.notify = () => {
        if (this.observationError || condition()) {
          clearTimeout(timeout)
          if (this.observationError) reject(this.observationError)
          else done()
        }
      }
      this.exit.then(() => {
        clearTimeout(timeout)
        if (this.spawnError || !condition())
          reject(
            this.spawnError ??
              new CoverageError('producer exited before evidence'),
          )
      })
    })
  }

  assertObservation(): void {
    if (this.observationError) throw this.observationError
  }

  observeFailure(error: CoverageError): void {
    this.observationError ??= error
    this.notify?.()
  }

  terminate(): void {
    if (
      this.spawnError ||
      this.child.exitCode !== null ||
      this.child.signalCode !== null
    )
      return
    this.cleanupRequested = true
    this.child.kill()
  }

  async finish(): Promise<void> {
    clearTimeout(this.deadline)
    await setImmediate()
    if (!this.cleanupRequested && !this.spawnError) {
      const code = this.child.exitCode
      const signal = this.child.signalCode
      if (code !== null || signal !== null)
        this.exitBeforeCleanup = this.naturalExit = { code, signal }
    }
    this.terminate()
    await this.exit
  }

  failure(cause: Error): CoverageError {
    return new ProducerFailure(cause, this.diagnostics())
  }

  private recordSpawnError(error: Error): void {
    this.spawnError = error
  }

  private diagnostics(): ProducerDiagnostics {
    return {
      group: this.run.group,
      files: [...this.run.files],
      naturalExit: this.naturalExit,
      exitBeforeCleanup: this.exitBeforeCleanup,
      cleanupRequested: this.cleanupRequested,
      finalExit: this.finalExit,
      stdoutTail: this.stdoutTail.toString(),
      stderrTail: this.stderrTail.toString(),
    }
  }
}
