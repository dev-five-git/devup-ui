import { spawn } from 'node:child_process'
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'

import { CoverageError } from './lcov'

export interface NativeCommand {
  readonly root: string
  readonly executable: string
  readonly args: readonly string[]
  readonly timeout: number
}

export interface NativeExit {
  readonly code: number | null
  readonly signal: NodeJS.Signals | null
  readonly stdout: string
  readonly stderr: string
  readonly stdoutTail: string
  readonly stderrTail: string
  readonly cleanupRequested: boolean
  readonly naturalExit: {
    readonly code: number | null
    readonly signal: NodeJS.Signals | null
  } | null
  readonly drained: boolean
}

export class NativeLaunchError extends CoverageError {
  constructor(
    override readonly cause: Error,
    readonly command: NativeCommand,
  ) {
    super(
      `native launch ${command.executable} ${command.args.join(' ')}: ${cause.message}`,
    )
  }
}

export async function nativeProcess(
  command: NativeCommand,
): Promise<NativeExit> {
  return new Promise((done, reject) => {
    const child = spawn(command.executable, [...command.args], {
      cwd: command.root,
      stdio: ['ignore', 'pipe', 'pipe'],
      env: process.env,
    })
    const stdout: Buffer[] = []
    const stderr: Buffer[] = []
    let cleanupRequested = false
    let launchError: Error | undefined
    child.stdout.on('data', (chunk: Buffer) => stdout.push(chunk))
    child.stderr.on('data', (chunk: Buffer) => stderr.push(chunk))
    child.once('error', (error) => {
      launchError = error
    })
    const deadline = setTimeout(() => {
      cleanupRequested = true
      child.kill()
    }, command.timeout)
    child.once('close', (code, signal) => {
      clearTimeout(deadline)
      if (launchError) {
        reject(new NativeLaunchError(launchError, command))
        return
      }
      const out = Buffer.concat(stdout)
      const err = Buffer.concat(stderr)
      done({
        code,
        signal,
        stdout: out.toString(),
        stderr: err.toString(),
        stdoutTail: out.subarray(-8192).toString(),
        stderrTail: err.subarray(-8192).toString(),
        cleanupRequested,
        naturalExit: cleanupRequested ? null : { code, signal },
        drained: true,
      })
    })
  })
}

export function nativeReport(directory: string): string {
  const file = join(directory, 'lcov.info')
  return existsSync(file) ? readFileSync(file, 'utf8') : ''
}
