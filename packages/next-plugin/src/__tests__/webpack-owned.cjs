const { spawn } = require('node:child_process')
const { existsSync, mkdtempSync, rmSync } = require('node:fs')
const { tmpdir } = require('node:os')
const { join } = require('node:path')
const { setTimeout: delay } = require('node:timers/promises')

const root = mkdtempSync(
  join(process.env.DEVUP_NEXT_FIXTURE_DIR || tmpdir(), 'w65-next-webpack-'),
)
const driver = join(__dirname, 'webpack-production.cjs')
const fault = process.argv[2] || ''
let outputClosed = false
// A cancelled caller's closed output pipes must not terminate the cleanup owner.
for (const stream of [process.stdout, process.stderr])
  stream.on('error', () => {
    outputClosed = true
  })
const record = (event, data) => {
  if (!outputClosed)
    console.info(`W65-LIFECYCLE ${JSON.stringify({ event, data })}`)
}

async function main() {
  if (process.platform === 'win32') {
    const quote = (value) =>
      `"${value.replace(/(\\*)"/g, '$1$1\\"').replace(/(\\+)$/g, '$1$1')}"`
    const config = Buffer.from(
      JSON.stringify({
        root,
        fault,
        owner: process.pid,
        caller: process.ppid,
        node: process.execPath,
        cwd: process.cwd(),
        command: [process.execPath, driver, fault].map(quote).join(' '),
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
        join(__dirname, 'webpack-supervise.ps1'),
        '-Config',
        config,
      ],
      { stdio: 'inherit', windowsHide: true },
    )
    // The foreground PS owner has its own deadline and monitors this handle.
    // Never time out/kill the sole job owner from spawnSync in the Bun test.
    process.exitCode = await new Promise((resolve, reject) => {
      supervisor.once('error', reject)
      supervisor.once('exit', (code) => resolve(code ?? 125))
    })
    return
  }

  // setsid is only for this referenced, awaited foreground process tree.
  const child = spawn(process.execPath, [driver, fault], {
    detached: true,
    stdio: 'inherit',
    env: { ...process.env, DEVUP_NEXT_FIXTURE_ROOT: root },
  })
  let outcome
  child.once('error', (error) => {
    outcome = 125
    console.error(error)
  })
  const exited = new Promise((resolve) =>
    child.once('exit', (code) => {
      outcome = code ?? 125
      resolve()
    }),
  )
  const signal = () => {
    outcome = 125
  }
  process.on('SIGTERM', signal)
  process.on('SIGINT', signal)
  const parent = process.ppid
  const started = Date.now()
  let deadline = started + 110000
  let reason = 'driver_exit'
  record('launch', { root, pid: child.pid })
  const send = (signal) => {
    try {
      process.kill(-child.pid, signal)
      return true
    } catch (error) {
      if (error.code === 'ESRCH') return false
      throw error
    }
  }
  try {
    while (outcome === undefined) {
      if (process.ppid !== parent || outputClosed) {
        outcome = 125
        reason = 'owner_exit'
        break
      }
      if (Date.now() >= deadline) {
        outcome = 124
        reason = 'deadline'
        break
      }
      if (fault === 'post-start-timeout' && existsSync(join(root, 'ready')))
        deadline = Math.min(deadline, Date.now() + 500)
      await delay(50)
    }
  } finally {
    send('SIGTERM')
    await delay(500)
    send('SIGKILL')
    const cleanupDeadline = Date.now() + 10000
    let active = send(0)
    while (active && Date.now() < cleanupDeadline) {
      await delay(20)
      active = send(0)
    }
    if (active) {
      console.error('Process group cleanup exceeded 10s; preserving fixture')
      outcome = 125
    } else {
      await exited
      record('tree_exit', { pgid: child.pid, active: [], reason })
      rmSync(root, { recursive: true, force: true })
      record('cleanup', { root, removed: true, exitCode: outcome })
    }
    process.removeListener('SIGTERM', signal)
    process.removeListener('SIGINT', signal)
  }
  process.exitCode = outcome
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 125
})
