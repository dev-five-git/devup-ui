import { describe, expect, it } from 'bun:test'

import config from '../../../../bindings/devup-ui-wasm/size-budget.json'
import { compare, validateConfig } from '../../../../wasm-size.js'

const metrics = [
  ['wasm', 'raw'],
  ['wasm', 'gzip'],
  ['glue', 'raw'],
  ['glue', 'gzip'],
] as const
const sizes = (value: number) => ({
  wasm: { raw: value, gzip: value },
  glue: { raw: value, gzip: value },
})

describe('relative WASM size policy', () => {
  it.each([9_000, 10_000, 10_001, 10_240])(
    'passes when all candidate metrics are %i bytes',
    (value) => {
      const base = sizes(10_000)
      const candidate = sizes(value)
      const result = compare(base, candidate, config)
      expect(result.passed).toBe(true)
      expect(result.metrics).toHaveLength(4)
      expect(result.metrics.map((metric) => metric.limitBytes)).toEqual([
        10_240, 10_240, 10_240, 10_240,
      ])
    },
  )

  it.each(metrics)(
    'fails only %s/%s when one metric exceeds the rounded bound',
    (artifact, kind) => {
      const candidate = sizes(10_000)
      candidate[artifact][kind] = 10_241
      const result = compare(sizes(10_000), candidate, config)
      expect(result.passed).toBe(false)
      expect(result.metrics.filter((metric) => !metric.passed)).toEqual([
        expect.objectContaining({
          artifact,
          kind,
          limitBytes: 10_240,
          deltaBytes: 241,
          deltaPercent: 2.41,
        }),
      ])
    },
  )

  it.each([
    [102_400, 103_424],
    [102_401, 104_448],
  ])('rounds %i base bytes up to %i', (before, limit) => {
    const result = compare(sizes(before), sizes(before), config)
    expect(result.metrics.map((metric) => metric.limitBytes)).toEqual([
      limit,
      limit,
      limit,
      limit,
    ])
  })

  it.each(metrics)(
    'uses the reviewed allowance only for %s/%s when overridden',
    (artifact, kind) => {
      const policy = structuredClone(config)
      policy[artifact][kind] = { growth: 0.1, roundBytes: 1 }
      const candidate = sizes(10_000)
      candidate[artifact][kind] = 11_000
      const result = compare(sizes(10_000), candidate, policy)
      expect(result.passed).toBe(true)
      expect(result.metrics.map((metric) => metric.limitBytes)).toEqual(
        metrics.map(([name, size]) =>
          name === artifact && size === kind ? 11_000 : 10_240,
        ),
      )
    },
  )

  it.each([
    { policy: undefined },
    { policy: null },
    { policy: {} },
    { policy: [] },
    { policy: { ...config, hiddenCap: 100 } },
    {
      policy: {
        wasm: { raw: 5779456, gzip: 2075648 },
        glue: { raw: 41984, gzip: 7168 },
      },
    },
  ])('rejects incomplete or absolute config %j', ({ policy }) => {
    expect(() => validateConfig(policy)).toThrow()
  })

  it.each([-1, NaN, Infinity, '0.01'])(
    'rejects invalid growth %s',
    (growth) => {
      const raw = { growth, roundBytes: 1024 }
      const policy = { ...config, wasm: { ...config.wasm, raw } }
      expect(() => validateConfig(policy)).toThrow()
    },
  )

  it.each([0, -1, 1.5, Infinity])(
    'rejects invalid rounding %s',
    (roundBytes) => {
      const gzip = { growth: 0.01, roundBytes }
      const policy = { ...config, glue: { ...config.glue, gzip } }
      expect(() => validateConfig(policy)).toThrow()
    },
  )

  it.each([0, -1, 1.5, NaN, Infinity, Number.MAX_SAFE_INTEGER])(
    'rejects invalid or unsafe measurement %s',
    (value) => {
      expect(() => compare(sizes(value), sizes(1), config)).toThrow()
    },
  )

  it.each([
    { base: undefined, candidate: sizes(1) },
    { base: sizes(1), candidate: undefined },
    { base: sizes(1), candidate: { wasm: { raw: 1, gzip: 1 } } },
    { base: { wasm: { raw: 1, gzip: 1 } }, candidate: sizes(1) },
  ])('rejects a missing measurement %j', ({ base, candidate }) => {
    expect(() => compare(base, candidate, config)).toThrow()
  })
})
