import { expect, it } from 'bun:test'

import { isMdxRecord } from '../mdx-pipeline'
import { turboWorkerFixture } from './mdx-turbo-worker-fixture'

it('matches actual stock Turbo worker marker bytes/maps across independent A-B-A builds', async () => {
  // Given
  const f = await turboWorkerFixture()
  try {
    const sources: string[] = []
    const pids = new Set<number>()
    // When: each CLI invocation replaces all native workers; preparation remains dormant.
    for (const text of ['worker-a', 'worker-b', 'worker-a']) {
      const records = await f.run(text)
      expect(records.length).toBeGreaterThan(0)
      // Then
      for (const record of records) {
        if (
          !isMdxRecord(record) ||
          !isMdxRecord(record.prepared) ||
          typeof record.pid !== 'number' ||
          typeof record.source !== 'string'
        )
          throw new TypeError('invalid receiving-worker observation')
        expect(record).toMatchObject({
          compilerPresent: false,
          modePresent: false,
          sourceMap: true,
          options: { format: 'mdx', remarkPlugins: [[f.plugin, { text }]] },
        })
        expect(record.source).toContain(text)
        if (record.prepared.error)
          console.info(
            JSON.stringify({
              compilerPresent: record.compilerPresent,
              modePresent: record.modePresent,
              sourceMap: record.sourceMap,
              error: record.prepared.error,
            }),
          )
        expect(record.prepared).toEqual({
          source: record.source,
          map: record.map,
        })
        expect(record.repeated).toEqual({
          source: record.source,
          map: record.map,
        })
        sources.push(record.source)
        pids.add(record.pid)
      }
    }
    expect(pids.size).toBeGreaterThanOrEqual(3)
    const last = sources.at(-1)
    if (!last) throw new TypeError('missing repeated worker output')
    expect(sources[0]).toBe(last)
  } finally {
    f.dispose()
  }
}, 400_000)

it.each([false, true])(
  'keeps real receiving-worker native/private marker cells disjoint privateFirst=%s',
  async (privateFirst) => {
    // Given
    const f = await turboWorkerFixture()
    try {
      // When
      const records = await f.run('worker-isolation', privateFirst)
      // Then
      expect(records.length).toBeGreaterThan(0)
      for (const record of records) {
        if (
          !isMdxRecord(record) ||
          !isMdxRecord(record.prepared) ||
          typeof record.source !== 'string'
        )
          throw new TypeError('missing isolation observation')
        expect(record.source).toContain('worker-isolation')
        expect(record.source).not.toContain('private-only')
        expect(record.prepared.source).toContain('private-only')
        expect(record.repeated).toEqual({
          source: record.source,
          map: record.map,
        })
      }
    } finally {
      f.dispose()
    }
  },
  150_000,
)
