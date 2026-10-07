import { execFile, spawn } from 'node:child_process'
import { once } from 'node:events'
import { promisify } from 'node:util'

import { readPrivateBytes } from './process-memory.mjs'

const execute = promisify(execFile)
const memoryLimit = 6 * 1024 ** 3

async function processSnapshot() {
  if (process.platform === 'win32') {
    const { stdout } = await execute('powershell.exe', [
      '-NoProfile',
      '-Command',
      'Get-CimInstance Win32_Process | ForEach-Object { @{ pid = [int]$_.ProcessId; parent = [int]$_.ParentProcessId; bytes = [double]$_.PrivatePageCount; born = $_.CreationDate.ToString("o") } } | ConvertTo-Json -Compress',
    ])
    return JSON.parse(stdout)
  }
  const { stdout } = await execute('ps', ['-eo', 'pid=,ppid='])
  return stdout
    .trim()
    .split('\n')
    .map((line) => {
      const [pid, parent] = line.trim().split(/\s+/).map(Number)
      return { pid, parent }
    })
}

async function privateBytes(row) {
  if (process.platform === 'win32') return row.bytes
  return readPrivateBytes(row.pid)
}

async function stopTree(pids) {
  const current = await processSnapshot()
  for (const [pid, born] of [...pids].reverse()) {
    if (process.platform === 'win32') {
      if (!born || current.find((row) => row.pid === pid)?.born !== born)
        continue
      try {
        await execute('taskkill.exe', ['/PID', String(pid), '/T', '/F'])
      } catch (error) {
        if (
          !(error instanceof Error) ||
          !('code' in error) ||
          ![128, 255].includes(error.code)
        )
          throw error
      }
    } else {
      try {
        process.kill(pid, 'SIGKILL')
      } catch (error) {
        if (
          !(error instanceof Error) ||
          !('code' in error) ||
          error.code !== 'ESRCH'
        )
          throw error
      }
    }
  }
}

export async function runGuarded(command, args, options = {}) {
  const child = spawn(command, args, {
    cwd: options.cwd,
    env: { ...process.env, CARGO_BUILD_JOBS: '2', ...options.env },
    stdio: ['ignore', 'pipe', 'pipe'],
  })
  const completion = once(child, 'close')
  child.stdout.on('data', (data) => {
    process.stdout.write(data)
    options.onOutput?.(data.toString())
  })
  child.stderr.on('data', (data) => {
    process.stderr.write(data)
    options.onOutput?.(data.toString())
  })
  const owned = new Map([[child.pid, undefined]])
  const started = performance.now()
  let peakBytes = 0
  let stopReason
  let polling = false
  let watchdogError
  const poll = async () => {
    if (polling) return
    polling = true
    try {
      const rows = await processSnapshot()
      if (child.exitCode === null && child.signalCode === null) {
        owned.set(child.pid, rows.find((row) => row.pid === child.pid)?.born)
      }
      let added
      do {
        added = false
        for (const row of rows) {
          const parentBorn = owned.get(row.parent)
          const parentOwned =
            owned.has(row.parent) &&
            (process.platform !== 'win32' ||
              (parentBorn !== undefined &&
                rows.find((parent) => parent.pid === row.parent)?.born ===
                  parentBorn))
          if (parentOwned && !owned.has(row.pid)) {
            owned.set(row.pid, row.born)
            added = true
          }
        }
      } while (added)
      const bytes = (
        await Promise.all(
          rows.filter((row) => owned.has(row.pid)).map(privateBytes),
        )
      ).reduce((total, value) => total + value, 0)
      peakBytes = Math.max(peakBytes, bytes)
      if (bytes > (options.memoryLimit ?? memoryLimit))
        stopReason = `private memory exceeded ${options.memoryLimit ?? memoryLimit} bytes`
      if (performance.now() - started > (options.timeoutMs ?? 600_000))
        stopReason = 'command timeout'
      if (options.signal?.aborted) stopReason = 'command cancelled'
      if (stopReason) await stopTree(owned)
    } catch (error) {
      watchdogError = error
      await stopTree(owned)
    } finally {
      polling = false
    }
  }
  let pendingPoll = poll()
  const timer = setInterval(() => {
    if (!polling) pendingPoll = poll()
  }, 1000)
  let code
  let signal
  try {
    await pendingPoll
    ;[code, signal] = await completion
  } finally {
    clearInterval(timer)
    await pendingPoll
    await stopTree(owned)
  }
  const result = {
    command,
    args,
    code,
    signal,
    peakBytes,
    durationMs: performance.now() - started,
    stopReason,
  }
  options.onResult?.(result)
  if (watchdogError) throw watchdogError
  if (stopReason)
    throw new Error(`FINDING: watchdog stopped ${command}: ${stopReason}`)
  if (code !== 0) throw new Error(`${command} exited ${code ?? signal}`)
  return result
}
