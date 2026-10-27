import { expect, it } from 'bun:test'

import { runWatchLifetime } from './watch-lifetime-child'

it.serial(
  'disposes watch observation deadlines before the post-completion hold',
  async () => {
    // Given the three unchanged real-watch cases and early native observers.
    // When their bodies and finally cleanup finish in one non-isolated child.
    const { status, output, records, owners } = runWatchLifetime()

    // Then all blue-CSS assertions pass without surviving or late deadlines.
    expect(status, output).toBe(0)
    expect(output).toMatch(/^\s*3 pass$/m)
    expect(output).toMatch(/^\s*0 fail$/m)
    expect(
      records
        .filter((row) => row.action === 'body-complete')
        .map((row) => row.complete),
    ).toEqual([1, 2, 3])
    const held = records.filter((row) => row.action === 'held')
    expect(held).toHaveLength(1)
    expect(held[0]?.hold).toBeGreaterThanOrEqual(11000)
    expect(
      records.filter((row) => row.action === 'final').map((row) => row.pending),
    ).toEqual([[]])
    for (const owner of owners) {
      expect(
        records.some((row) => row.action === 'register' && row.owner === owner),
      ).toBe(true)
    }
    expect(
      records.filter((row) => row.action === 'fire' && row.afterFinal),
    ).toEqual([])
    expect(records.filter((row) => row.action === 'restored')).toHaveLength(1)
  },
  120000,
)
