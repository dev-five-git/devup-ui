import assert from 'node:assert/strict'
import { test } from 'node:test'

import { readPrivateBytes } from './process-memory.mjs'

test('counts private memory and excludes shared pages when a proc read succeeds', async () => {
  const reader = async () =>
    'Private_Clean: 2 kB\nPrivate_Dirty: 3 kB\nPrivate_Hugetlb: 4 kB\nShared_Clean: 20 kB\n'
  assert.equal(await readPrivateBytes(42, reader), 9 * 1024)
})

for (const code of ['ENOENT', 'ESRCH']) {
  test(`counts an exited process as zero when a proc read reports ${code}`, async () => {
    const reader = async () => {
      throw Object.assign(new Error('process exited'), { code })
    }
    assert.equal(await readPrivateBytes(42, reader), 0)
  })
}

test('propagates permission failures instead of undercounting a live process', async () => {
  const error = Object.assign(new Error('permission denied'), {
    code: 'EACCES',
  })
  const reader = async () => {
    throw error
  }
  await assert.rejects(readPrivateBytes(42, reader), error)
})
