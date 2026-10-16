import assert from 'node:assert/strict'
import { resolve } from 'node:path'

import type { NativeEvent, Row, Snapshot } from './native-webpack-types.ts'

export function assertRetained(before: Snapshot, after: Snapshot): void {
  for (const [bucket, slots] of Object.entries(before.classes))
    for (const [key, value] of Object.entries(slots))
      assert.equal(
        after.classes[bucket]?.[key],
        value,
        `old class slot ${bucket}/${key}`,
      )
  for (const [key, value] of Object.entries(before.files))
    assert.equal(after.files[key], value, `old file slot ${key}`)
}
export function assertPhysicalOrder(rows: readonly Row[], root: string): void {
  const physical = rows.map(([id]) => resolve(root, id).replaceAll('\\', '/'))
  assert.deepEqual(physical, [...physical].sort())
}
export function assertReserved(records: readonly NativeEvent[]): void {
  const reservations = records.filter((entry) => entry.kind === 'reserve')
  assert(reservations.length > 0, 'genuine reservation must execute')
  assert(reservations.some((entry) => entry.args[0].length > 0))
  for (const entry of records.filter((entry) => entry.kind === 'extract'))
    assert.equal(entry.after, 0, `residual after ${entry.args[0]}`)
}
