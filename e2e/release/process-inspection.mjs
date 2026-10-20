import { execFile, spawn } from 'node:child_process'
import { readdir, readFile } from 'node:fs/promises'
import { promisify } from 'node:util'

import { readPrivateBytes } from './process-memory.mjs'

const execute = promisify(execFile)
const windowsFields =
  '@{ pid = [int]$_.ProcessId; parent = [int]$_.ParentProcessId; bytes = [double]$_.PrivatePageCount; born = $_.CreationDate.ToString("o"); bornOrder = $_.CreationDate.ToUniversalTime().Ticks.ToString() }'

export function parseProcStat(stat) {
  // comm can itself contain spaces and parentheses; fields start after its LAST ).
  const closing = stat.lastIndexOf(')')
  const pid = Number(stat.slice(0, stat.indexOf(' ')))
  const fields = stat
    .slice(closing + 1)
    .trim()
    .split(/\s+/)
  const parent = Number(fields[1])
  const born = fields[19]
  if (
    closing < 0 ||
    !Number.isSafeInteger(pid) ||
    pid <= 0 ||
    !Number.isSafeInteger(parent) ||
    !/^\d+$/.test(born ?? '')
  )
    throw new Error('Invalid /proc process identity')
  if (fields[0] === 'Z' || fields[0] === 'X') return undefined
  return { pid, parent, born, bornOrder: born }
}

async function windowsSnapshot(filter = '') {
  const { stdout } = await execute(
    'powershell.exe',
    [
      '-NoProfile',
      '-Command',
      `Get-CimInstance Win32_Process ${filter} | ForEach-Object { ${windowsFields} } | ConvertTo-Json -Compress`,
    ],
    { timeout: 5000, maxBuffer: 8 * 1024 ** 2 },
  )
  return stdout.trim() ? [].concat(JSON.parse(stdout)) : []
}

async function inspect(pid) {
  if (!Number.isSafeInteger(pid) || pid <= 0)
    throw new Error('Invalid process PID')
  if (process.platform === 'win32')
    return (await windowsSnapshot(`-Filter "ProcessId = ${pid}"`))[0]
  try {
    return parseProcStat(await readFile(`/proc/${pid}/stat`, 'utf8'))
  } catch (error) {
    if (
      error instanceof Error &&
      'code' in error &&
      ['ENOENT', 'ESRCH'].includes(error.code)
    )
      return undefined
    throw error
  }
}

async function snapshot() {
  if (process.platform === 'win32') return windowsSnapshot()
  const pids = (await readdir('/proc')).filter((name) => /^\d+$/.test(name))
  return (await Promise.all(pids.map((pid) => inspect(Number(pid))))).filter(
    Boolean,
  )
}

async function kill(pid) {
  process.kill(pid, 'SIGKILL')
}

export const processRuntime = {
  spawn,
  inspect,
  snapshot,
  kill,
  privateBytes: (row) =>
    process.platform === 'win32' ? row.bytes : readPrivateBytes(row.pid),
}
