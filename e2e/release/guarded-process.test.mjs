import assert from 'node:assert/strict'
import { EventEmitter } from 'node:events'
import { test } from 'node:test'

import { CommandExitError, runGuarded } from './guarded-process.mjs'

function fixture() {
  const child = Object.assign(new EventEmitter(), {
    pid: 41001,
    exitCode: null,
    signalCode: null,
    stdout: new EventEmitter(),
    stderr: new EventEmitter(),
  })
  const root = {
    pid: child.pid,
    parent: 0,
    born: '100',
    bornOrder: '100',
    bytes: 1,
  }
  let rows = [root]
  const killed = []
  const sampled = []
  const results = []
  const close = () => {
    if (child.exitCode !== null) return
    child.exitCode = 0
    child.emit('close', 0, null)
  }
  const runtime = {
    spawn: () => child,
    snapshot: async () => rows,
    inspect: async (pid) => rows.find((row) => row.pid === pid),
    privateBytes: async (row) => {
      sampled.push(row.born)
      return row.bytes
    },
    kill: async (pid) => {
      killed.push(pid)
      rows = rows.filter((row) => row.pid !== pid)
      if (pid === child.pid) close()
    },
  }
  return {
    child,
    root,
    runtime,
    killed,
    sampled,
    results,
    close,
    setRows: (next) => {
      rows = next
    },
    run: () =>
      runGuarded('controlled', [], {
        runtime,
        pollIntervalMs: 5,
        drainTimeoutMs: 50,
        onResult: (result) => results.push(result),
      }),
  }
}

test('does not sample or kill a reused child or adopt a reused parent descendant', async () => {
  // Given: one owned child survives while another parent PID is reused.
  const f = fixture()
  const old = {
    pid: 41002,
    parent: f.root.pid,
    born: '101',
    bornOrder: '101',
    bytes: 2,
  }
  const survivor = {
    pid: 41003,
    parent: f.root.pid,
    born: '102',
    bornOrder: '102',
    bytes: 3,
  }
  const replacement = {
    ...old,
    parent: 0,
    born: '999',
    bornOrder: '999',
    bytes: 1000,
  }
  const outsider = {
    pid: 41004,
    parent: old.pid,
    born: '1000',
    bornOrder: '1000',
    bytes: 2000,
  }
  let polls = 0
  f.runtime.snapshot = async () => {
    polls += 1
    const rows =
      polls === 1
        ? [f.root, old, survivor]
        : [f.root, replacement, survivor, outsider]
    f.setRows(rows)
    if (polls === 2) setImmediate(f.close)
    return rows
  }
  // When: the supervisor polls both generations then cleans up.
  await f.run()
  // Then: only the surviving owned generations are charged or signalled.
  assert.equal(f.results[0].peakBytes, 6)
  assert.ok(!f.sampled.includes('999') && !f.sampled.includes('1000'))
  assert.ok(!f.killed.includes(old.pid) && !f.killed.includes(outsider.pid))
  assert.ok(f.killed.includes(survivor.pid))
})

test('does not rebind an established root to a replacement generation', async () => {
  const f = fixture()
  let polls = 0
  f.runtime.snapshot = async () => {
    polls += 1
    const rows =
      polls === 1
        ? [f.root]
        : [{ ...f.root, born: '999', bornOrder: '999', bytes: 1000 }]
    f.setRows(rows)
    if (polls === 2) setImmediate(f.close)
    return rows
  }
  await f.run()
  assert.equal(f.results[0].peakBytes, 1)
  assert.deepEqual(f.killed, [])
})

for (const phase of ['initial', 'later']) {
  test(`cleans known identities and emits evidence when ${phase} snapshots reject`, async () => {
    const f = fixture()
    const failure = new Error(`${phase} snapshot failure`)
    let polls = 0
    f.runtime.snapshot = async () => {
      polls += 1
      if (phase === 'later' && polls === 1) return [f.root]
      // Bound the old implementation's wait without signalling any real process.
      setTimeout(f.close, 30)
      throw failure
    }
    await assert.rejects(
      f.run(),
      (error) => error === failure || error.errors?.includes(failure),
    )
    assert.ok(f.killed.includes(f.root.pid))
    assert.equal(f.results.length, 1)
  })
}

test('attempts remaining identities and preserves evidence when one kill rejects', async () => {
  const f = fixture()
  const failure = new Error('individual kill failure')
  const descendant = {
    pid: 41002,
    parent: f.root.pid,
    born: '101',
    bornOrder: '101',
    bytes: 2,
  }
  f.setRows([f.root, descendant])
  f.runtime.privateBytes = async () => {
    setImmediate(f.close)
    return 1
  }
  const kill = f.runtime.kill
  f.runtime.kill = async (pid) => {
    if (pid === descendant.pid) throw failure
    await kill(pid)
  }
  await assert.rejects(
    f.run(),
    (error) => error === failure || error.errors?.includes(failure),
  )
  assert.ok(f.killed.includes(f.root.pid))
  assert.equal(f.results.length, 1)
})

test('preserves primary and cleanup failures without accepting a compiler exit', async () => {
  const f = fixture()
  const primary = new Error('snapshot rejected')
  const cleanup = new Error('kill rejected')
  f.runtime.snapshot = async () => {
    throw primary
  }
  f.runtime.kill = async () => {
    setImmediate(f.close)
    throw cleanup
  }
  await assert.rejects(
    f.run(),
    (error) =>
      error instanceof AggregateError &&
      !(error instanceof CommandExitError) &&
      error.errors.includes(primary) &&
      error.errors.includes(cleanup),
  )
  assert.equal(f.results.length, 1)
  assert.ok(f.results[0].errors.includes(primary.message))
  assert.ok(f.results[0].errors.includes(cleanup.message))
})

test('bounds drain failure and still emits result evidence when termination does not drain', async () => {
  const f = fixture()
  f.runtime.privateBytes = async () => {
    setImmediate(f.close)
    return 1
  }
  f.runtime.kill = async () => {}
  await assert.rejects(
    f.run(),
    (error) =>
      error instanceof AggregateError &&
      error.errors.some((cause) => /drain timed out/.test(cause.message)),
  )
  assert.equal(f.results.length, 1)
})

test('bounds a stuck poll and cleans known identities before surfacing failure', async () => {
  const f = fixture()
  f.runtime.snapshot = () => new Promise(() => {})
  await assert.rejects(
    f.run(),
    (error) =>
      error instanceof AggregateError &&
      error.errors.some((cause) => /timed out/.test(cause.message)),
  )
  assert.deepEqual(f.killed, [f.root.pid])
  assert.equal(f.results.length, 1)
})
