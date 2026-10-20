import assert from 'node:assert/strict'
import { EventEmitter } from 'node:events'
import { test } from 'node:test'

import { runGuarded } from './guarded-process.mjs'

function windowsFixture() {
  const root = {
    pid: 42001,
    parent: 0,
    born: '2026-10-19T12:00:00.0000002+00:00',
    bornOrder: '639280080000000002',
    bytes: 1,
  }
  const child = {
    pid: 42002,
    parent: root.pid,
    born: '2026-10-19T12:00:00.0000003+00:00',
    bornOrder: '639280080000000003',
    bytes: 2,
  }
  const childProcess = Object.assign(new EventEmitter(), {
    pid: root.pid,
    stdout: new EventEmitter(),
    stderr: new EventEmitter(),
  })
  let rows = [root, child]
  const events = []
  const sampled = []
  const results = []
  const runtime = {
    spawn: () => childProcess,
    snapshot: async () => rows,
    inspect: async (pid) => {
      events.push(['inspect', pid])
      return rows.find((row) => row.pid === pid)
    },
    privateBytes: async (row) => {
      sampled.push(row.pid)
      if (row.pid === child.pid)
        setImmediate(() => childProcess.emit('close', 0, null))
      return row.bytes
    },
    kill: async (pid) => {
      events.push(['kill', pid])
      rows = rows.filter((row) => row.pid !== pid)
    },
  }
  return {
    root,
    child,
    runtime,
    events,
    sampled,
    results,
    setRows: (next) => {
      rows = next
    },
    run: () =>
      runGuarded('windows-controlled', [], {
        runtime,
        drainTimeoutMs: 50,
        onResult: (result) => results.push(result),
      }),
  }
}

test('rejects an older stable Windows stale-parent outsider and its descendant', async () => {
  // Given: all generations stay stable, but an older outsider retains the root PID.
  const f = windowsFixture()
  const outsider = {
    ...f.child,
    pid: 42003,
    born: '2026-10-19T12:00:00.0000001+00:00',
    bornOrder: '639280080000000001',
    bytes: 1000,
  }
  const descendant = {
    ...f.child,
    pid: 42004,
    parent: outsider.pid,
    bytes: 2000,
  }
  f.setRows([f.root, outsider, descendant, f.child])
  // When: discovery, memory sampling and cleanup run through the supervisor.
  const result = await f.run()
  // Then: only the root and genuinely newer child are charged and cleaned.
  assert.equal(result.peakBytes, 3)
  assert.deepEqual(f.sampled, [f.root.pid, f.child.pid])
  assert.deepEqual(
    f.events.filter(([event]) => event === 'kill'),
    [
      ['kill', f.child.pid],
      ['kill', f.root.pid],
    ],
  )
  assert.deepEqual(result.errors, [])
})

for (const outcome of ['absent', 'replacement']) {
  test(`drains without error when native kill ESRCH finds the generation ${outcome}`, async () => {
    // Given: a matching pre-signal generation and locale-independent native error.
    const f = windowsFixture()
    const kill = f.runtime.kill
    const missing = Object.assign(new Error('임의의 한국어 진단'), {
      code: 'ESRCH',
      syscall: 'kill',
    })
    f.runtime.kill = async (pid) => {
      await kill(pid)
      if (pid === f.child.pid) {
        if (outcome === 'replacement')
          f.setRows([f.root, { ...f.child, born: 'replacement' }])
        throw missing
      }
    }
    // When: cleanup encounters that specific missing-process outcome.
    const result = await f.run()
    // Then: it re-inspects the expected generation and still cleans the root.
    const signal = f.events.findIndex(([event]) => event === 'kill')
    assert.deepEqual(f.events.slice(signal - 1, signal + 3), [
      ['inspect', f.child.pid],
      ['kill', f.child.pid],
      ['inspect', f.child.pid],
      ['inspect', f.root.pid],
    ])
    assert.deepEqual(
      f.events.filter(([event]) => event === 'kill'),
      [
        ['kill', f.child.pid],
        ['kill', f.root.pid],
      ],
    )
    assert.deepEqual(result.errors, [])
    assert.equal(result.code, 0)
  })
}

for (const [outcome, code, syscall] of [
  ['live', 'ESRCH', 'kill'],
  ['permission', 'EPERM', 'kill'],
  ['inspection', 'ESRCH', 'kill'],
  ['other-native', 'EINVAL', 'kill'],
  ['numeric128', 128, 'kill'],
  ['numeric255', 255, 'kill'],
  ['numeric1', 1, 'kill'],
  ['non-kill', 'ESRCH', 'read'],
]) {
  test(`preserves native kill failure when the outcome is ${outcome}`, async () => {
    // Given: only actual kill ESRCH can trigger immediate generation reinspection.
    const f = windowsFixture()
    const kill = f.runtime.kill
    const inspect = f.runtime.inspect
    const inspectionFailure = new Error('reinspection denied')
    const failure = Object.assign(new Error('임의의 한국어 진단'), {
      code,
      syscall,
      cmd: `taskkill.exe /PID ${f.child.pid} /F`,
      stderr: `ERROR: The process "${f.child.pid}" not found.\r\n`,
    })
    let attempted = false
    f.runtime.inspect = async (pid) => {
      if (attempted && pid === f.child.pid) {
        if (outcome === 'inspection') {
          f.events.push(['inspection-failure', pid])
          throw inspectionFailure
        }
        if (outcome === 'live') {
          const current = await inspect(pid)
          f.setRows([f.root])
          return current
        }
      }
      return inspect(pid)
    }
    f.runtime.kill = async (pid) => {
      if (pid !== f.child.pid) return kill(pid)
      f.events.push(['kill', pid])
      attempted = true
      if (outcome !== 'live' && outcome !== 'inspection') f.setRows([f.root])
      throw failure
    }
    // When: cleanup handles the error and continues to other owned processes.
    await assert.rejects(f.run(), (error) => {
      assert.ok(error instanceof AggregateError)
      assert.ok(error.errors.includes(failure))
      if (outcome === 'inspection')
        assert.ok(error.errors.includes(inspectionFailure))
      return true
    })
    // Then: exact errors survive later absence and remaining root cleanup runs.
    if (outcome === 'live' || outcome === 'inspection') {
      const signal = f.events.findIndex(([event]) => event === 'kill')
      assert.deepEqual(f.events[signal + 1], [
        outcome === 'live' ? 'inspect' : 'inspection-failure',
        f.child.pid,
      ])
    }
    assert.deepEqual(
      f.events.filter(([event]) => event === 'kill'),
      [
        ['kill', f.child.pid],
        ['kill', f.root.pid],
      ],
    )
  })
}

test('retains pre-signal inspection ESRCH without signalling or missing-kill classification', async () => {
  // Given: discovery and memory succeed, but the child's pre-signal read fails.
  const f = windowsFixture()
  const inspect = f.runtime.inspect
  const failure = Object.assign(new Error('검사 실패'), {
    code: 'ESRCH',
    syscall: 'kill',
  })
  let childReads = 0
  f.runtime.inspect = async (pid) => {
    if (pid === f.child.pid && ++childReads === 4) {
      f.events.push(['inspection-failure', pid])
      f.setRows([f.root])
      throw failure
    }
    return inspect(pid)
  }
  // When: cleanup validates each identity before attempting its signal.
  await assert.rejects(f.run(), (error) => {
    assert.ok(error instanceof AggregateError)
    assert.deepEqual(error.errors, [failure])
    return true
  })
  // Then: no child signal or immediate missing-kill reinspection occurs.
  const failed = f.events.findIndex(([event]) => event === 'inspection-failure')
  assert.deepEqual(f.events.slice(failed, failed + 3), [
    ['inspection-failure', f.child.pid],
    ['inspect', f.root.pid],
    ['kill', f.root.pid],
  ])
  assert.deepEqual(
    f.events.filter(([event]) => event === 'kill'),
    [['kill', f.root.pid]],
  )
})
