import assert from 'node:assert/strict'
import { test } from 'node:test'

import { parseProcStat } from './process-inspection.mjs'
import { ProcessOwnership } from './process-ownership.mjs'

test('parses Linux start ticks after a command containing spaces and closing parentheses', () => {
  const fields = ['S', '11', ...Array(17).fill('0'), '12345678901234', '0']
  assert.deepEqual(
    parseProcStat(`42 (a ) tricky (name)) ${fields.join(' ')}`),
    {
      pid: 42,
      parent: 11,
      born: '12345678901234',
      bornOrder: '12345678901234',
    },
  )
  assert.equal(
    parseProcStat(`42 (zombie) ${['Z', ...fields.slice(1)].join(' ')}`),
    undefined,
  )
  assert.throws(() => parseProcStat('42 (bad) S 11'))
})

test('discards a memory sample when generation changes during the read', async () => {
  const original = { pid: 42, parent: 0, born: '123', bornOrder: '123' }
  let current = original
  const owner = new ProcessOwnership({
    inspect: async () => current,
    privateBytes: async () => {
      current = { ...original, born: '456', bornOrder: '456' }
      return 999
    },
  })
  await owner.establish(42)
  assert.equal(await owner.memory(), 0)
  assert.equal(owner.owned.size, 0)
})

test('rechecks identity immediately before signalling, independently of global snapshots', async () => {
  const original = { pid: 42, parent: 0, born: '123', bornOrder: '123' }
  let current = original
  const killed = []
  const owner = new ProcessOwnership({
    inspect: async () => current,
    kill: async (pid) => killed.push(pid),
  })
  await owner.establish(42)
  current = { ...original, born: '456', bornOrder: '456' }
  assert.deepEqual(await owner.stop(50), [])
  assert.deepEqual(killed, [])
})

test('does not adopt descendants when a known parent generation disappears', async () => {
  const root = { pid: 42, parent: 0, born: '123', bornOrder: '123' }
  const old = { pid: 43, parent: 42, born: '124', bornOrder: '124' }
  const survivor = { pid: 44, parent: 43, born: '125', bornOrder: '125' }
  let rows = [root, old, survivor]
  const owner = new ProcessOwnership({
    inspect: async (pid) => rows.find((row) => row.pid === pid),
  })
  await owner.establish(42)
  await owner.discover(rows)
  rows = [
    root,
    { ...old, parent: 0, born: '999', bornOrder: '999' },
    survivor,
    { pid: 45, parent: 43, born: '1000', bornOrder: '1000' },
  ]
  await owner.discover(rows)
  assert.deepEqual([...owner.owned.keys()], [42, 44])
})

test('rechecks parent generation after candidate inspection before adopting descendants', async () => {
  const root = { pid: 42, parent: 0, born: '123', bornOrder: '123' }
  const candidate = { pid: 43, parent: 42, born: '124', bornOrder: '124' }
  let parent = root
  const owner = new ProcessOwnership({
    inspect: async (pid) => {
      if (pid === root.pid) return parent
      parent = { ...root, born: '999', bornOrder: '999' }
      return candidate
    },
  })
  await owner.establish(root.pid)
  await owner.discover([root, candidate])
  assert.equal(owner.owned.size, 0)
})

for (const ticks of ['9', '90071992547409929']) {
  test(`allows same-tick and numerically newer Linux children at tick ${ticks}`, async () => {
    // Given: exact ticks straddle a decimal width or exceed Number precision.
    const root = { pid: 42, parent: 0, born: ticks, bornOrder: ticks }
    const rows = [
      root,
      { pid: 43, parent: 42, born: ticks, bornOrder: ticks },
      {
        pid: 44,
        parent: 42,
        born: String(BigInt(ticks) + 1n),
        bornOrder: String(BigInt(ticks) + 1n),
      },
      {
        pid: 45,
        parent: 42,
        born: String(BigInt(ticks) - 1n),
        bornOrder: String(BigInt(ticks) - 1n),
      },
    ]
    const owner = new ProcessOwnership({
      inspect: async (pid) => rows.find((row) => row.pid === pid),
    })
    await owner.establish(root.pid)
    // When: ancestry is admitted using the platform's exact creation order.
    await owner.discover(rows)
    // Then: equality is allowed, numeric newer is allowed, numeric older is not.
    assert.deepEqual([...owner.owned.keys()], [42, 43, 44])
  })
}
