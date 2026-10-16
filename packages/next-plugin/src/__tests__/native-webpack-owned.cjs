const assert = require('node:assert/strict')
const { spawn } = require('node:child_process')
const { join } = require('node:path')
const { setTimeout: delay } = require('node:timers/promises')

const deadline = Number(process.argv[2])
const args = process.argv.slice(3)
assert(Number.isSafeInteger(deadline) && deadline > 0 && args.length > 0)

async function main() {
  if (process.platform === 'win32') {
    /** @param {string} value */
    const quote = (value) =>
      `"${value.replace(/(\\*)"/g, '$1$1\\"').replace(/(\\+)$/g, '$1$1')}"`
    const config = Buffer.from(
      JSON.stringify({
        owner: process.pid,
        caller: process.ppid,
        cwd: process.cwd(),
        command: [process.execPath, ...args].map(quote).join(' '),
        deadline,
      }),
    ).toString('base64')
    const supervisor = spawn(
      'powershell.exe',
      [
        '-NoLogo',
        '-NoProfile',
        '-NonInteractive',
        '-ExecutionPolicy',
        'Bypass',
        '-File',
        join(__dirname, 'native-webpack-supervise.ps1'),
        '-Config',
        config,
      ],
      { stdio: 'inherit', windowsHide: true },
    )
    process.exitCode = await new Promise((resolve, reject) => {
      supervisor.once('error', reject)
      supervisor.once('close', (code) => resolve(code ?? 125))
    })
    return
  }

  // Referenced and awaited foreground group, never an unowned background task.
  const child = spawn(process.execPath, args, {
    detached: true,
    stdio: 'inherit',
  })
  /** @type {number | undefined} */
  let outcome
  const cancel = () => {
    outcome = 125
  }
  process.on('SIGTERM', cancel)
  process.on('SIGINT', cancel)
  let outputClosed = false
  /** @param {Error} error */
  const outputError = (error) => {
    if (!('code' in error && error.code === 'EPIPE')) throw error
    outputClosed = true
    cancel()
    process.exitCode = 125
  }
  // A cancelled caller's closed pipes must not interrupt group cleanup.
  process.stdout.on('error', outputError)
  process.stderr.on('error', outputError)
  const exited = new Promise((resolve, reject) => {
    child.once('error', reject)
    child.once('exit', (code) => {
      outcome ??= code ?? 125
      resolve(undefined)
    })
  })
  assert(child.pid)
  const pgid = child.pid
  /** @param {NodeJS.Signals | 0} signal */
  function send(signal) {
    try {
      process.kill(-pgid, signal)
      return true
    } catch (error) {
      if (error instanceof Error && 'code' in error && error.code === 'ESRCH')
        return false
      throw error
    }
  }
  const parent = process.ppid
  const end = Date.now() + deadline
  try {
    while (outcome === undefined) {
      if (process.ppid !== parent) {
        outcome = 125
        break
      }
      if (Date.now() >= end) {
        outcome = 124
        break
      }
      await delay(50)
    }
  } finally {
    send('SIGKILL')
    await exited
    const drain = Date.now() + 10000
    while (send(0)) {
      assert(Date.now() < drain, 'Native process group drain deadline')
      await delay(20)
    }
    process.removeListener('SIGTERM', cancel)
    process.removeListener('SIGINT', cancel)
    if (!outputClosed)
      console.info(
        `NATIVE-LIFECYCLE ${JSON.stringify({ event: 'tree_exit', pgid, active: [], exitCode: outcome })}`,
      )
  }
  process.exitCode = outcome
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 125
})
