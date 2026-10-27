import { expect, it } from 'bun:test'

import { runWatchLifetime } from './watch-lifetime-child'

it.serial(
  'unregisters real Vite Outputs before the post-completion hold',
  async () => {
    // Given early pass-through weak observers and the original watch scenarios.
    // When the non-isolated native child completes and remains alive for 11 seconds.
    const { status, output, records } = runWatchLifetime()

    // Then native Outputs are explicitly freed, never left to finalization.
    expect(status, output).toBe(0)
    expect(output).toMatch(/^\s*3 pass$/m)
    expect(output).toMatch(/^\s*0 fail$/m)
    expect(
      records
        .filter((row) => row.action === 'body-complete')
        .map((row) => row.complete),
    ).toEqual([1, 2, 3])
    const registered = records
      .filter((row) => row.action === 'output-register')
      .map((row) => row.id)
    expect(registered.length).toBeGreaterThan(0)
    expect(
      records
        .filter((row) => row.action === 'output-unregister')
        .map((row) => row.id),
    ).toEqual(registered)
    expect(
      records
        .filter((row) => row.action === 'output-final')
        .map((row) => row.pending),
    ).toEqual([[]])
    expect(records.filter((row) => row.action === 'output-finalize')).toEqual(
      [],
    )
    const held = records.filter((row) => row.action === 'held')
    expect(held).toHaveLength(1)
    expect(held[0]?.hold).toBeGreaterThanOrEqual(11000)
    expect(records.filter((row) => row.action === 'restored')).toHaveLength(1)
  },
  120000,
)
