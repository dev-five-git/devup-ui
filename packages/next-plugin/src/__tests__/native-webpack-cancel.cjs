const assert = require('node:assert/strict')
const { spawn } = require('node:child_process')
const {
  closeSync,
  mkdtempSync,
  openSync,
  readFileSync,
  rmSync,
} = require('node:fs')
const { tmpdir } = require('node:os')
const { join } = require('node:path')
const { setTimeout: delay } = require('node:timers/promises')

async function main() {
  const signal = process.argv[2]
  assert(signal === 'SIGTERM' || signal === 'SIGINT')
  const closed = process.argv[3] === 'closed'
  const root = mkdtempSync(join(tmpdir(), 'native-owner-cancel-'))
  const output = join(root, 'stdout')
  const fd = openSync(output, 'wx')
  const owner = spawn(
    process.execPath,
    [
      join(__dirname, 'native-webpack-owned.cjs'),
      '30000',
      join(__dirname, 'webpack-lifecycle-child.cjs'),
    ],
    { stdio: ['ignore', closed ? 'pipe' : fd, 'pipe'] },
  )
  closeSync(fd)
  let stdout = ''
  let stderr = ''
  /** @type {Set<number>} */
  const pids = new Set()
  /** @type {Promise<{code: number | null, signal: NodeJS.Signals | null}>} */
  const complete = new Promise((resolve, reject) => {
    owner.once('error', reject)
    owner.once('close', (code, signal) => resolve({ code, signal }))
  })
  /** @type {ReturnType<typeof setTimeout> | undefined} */
  let timer
  const timeout = new Promise((_resolve, reject) => {
    timer = setTimeout(
      () => reject(new Error('Owner cancellation deadline')),
      12000,
    )
  })
  if (closed) {
    assert(owner.stdout)
    owner.stdout.on('data', (chunk) => {
      stdout += String(chunk)
    })
  }
  assert(owner.stderr)
  owner.stderr.on('data', (chunk) => {
    stderr += String(chunk)
  })
  /** @param {number} pid @param {NodeJS.Signals | 0} signal */
  function send(pid, signal) {
    try {
      process.kill(pid, signal)
      return true
    } catch (error) {
      if (error instanceof Error && 'code' in error && error.code === 'ESRCH')
        return false
      throw error
    }
  }
  try {
    // Given: both real fault descendants are alive before cancelling their owner.
    const readyEnd = Date.now() + 10000
    while (pids.size !== 2) {
      if (!closed) stdout = readFileSync(output, 'utf8')
      for (const match of stdout.matchAll(/W65-FAULT-PID (\d+)/g))
        pids.add(Number(match[1]))
      assert(Date.now() < readyEnd, 'Fault descendants did not become ready')
      if (pids.size !== 2) await Promise.race([delay(20), timeout])
    }
    assert.equal(pids.size, 2)
    for (const pid of pids) assert(send(pid, 0))
    // When: signal only the owner, never the fault child or its process group.
    assert(owner.kill(signal))
    if (closed) {
      owner.stdout?.destroy()
      owner.stderr.destroy()
    }
    const end = Date.now() + 10000
    const result = await Promise.race([
      complete,
      timeout.then(() => assert.fail('Cancellation timed out')),
    ])
    while ([...pids].some((pid) => send(pid, 0))) {
      assert(
        Date.now() < end,
        'Owner cancellation left a native descendant alive',
      )
      await delay(20)
    }
    if (!closed) stdout = readFileSync(output, 'utf8')
    // Then: cancellation is failure, not the 30s command timeout or success.
    if (process.platform === 'win32') {
      assert.equal(result.code, null)
      assert.equal(result.signal, signal)
    } else {
      assert.equal(result.code, 125, stderr)
      assert.equal(result.signal, null)
      if (!closed) assert.match(stdout, /"active":\[\]/)
      for (const pid of pids)
        assert.equal(send(-pid, 0), false, 'Native process group remains')
    }
    console.info(
      JSON.stringify({
        event: 'owner_cancel',
        owner: owner.pid,
        signal,
        closed,
        pids: [...pids],
        active: [],
        result,
        stdout,
        stderr,
      }),
    )
  } finally {
    clearTimeout(timer)
    // A failing regression must not strand the very descendants it detects.
    if (process.platform !== 'win32') {
      for (const pid of pids) send(-pid, 'SIGKILL')
    }
    for (const pid of pids) if (send(pid, 0)) send(pid, 'SIGKILL')
    owner.kill('SIGKILL')
    await complete
    rmSync(root, { recursive: true, force: true })
  }
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
