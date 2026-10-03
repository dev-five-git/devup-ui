import { describe, expect, it } from 'bun:test'

import {
  BYTES_TOLERANCE,
  CSS_MIN_SLACK,
  evaluate,
  KEPT_CALIBRATION_RUNS,
  median,
  MIN_TIME_TOLERANCE,
  nextBaseline,
  relativeSpread,
  report,
  roundOrder,
  timeRatio,
  timeTolerance,
} from '../../../../benchmark-gate.js'

interface Row {
  samples: number[]
  median: number
  min: number
  max: number
  bytes: number
  cssBytes: number
}
interface Run {
  samples: number
  rows: Record<string, Row>
}

const manifest = {
  samples: 3,
  targets: [
    { name: 'tailwind', group: 'webpack' },
    { name: 'tailwind-turbo', group: 'turbopack' },
    { name: 'devup-ui', group: 'webpack', gate: true },
    { name: 'devup-ui-turbo', group: 'turbopack', gate: true },
    { name: 'mui', group: 'webpack' },
  ],
  controls: { webpack: 'tailwind', turbopack: 'tailwind-turbo' },
}

function row(seconds: number, bytes: number, cssBytes: number): Row {
  return {
    samples: [seconds, seconds, seconds],
    median: seconds,
    min: seconds,
    max: seconds,
    bytes,
    cssBytes,
  }
}

/** A run of the normal speed: Tailwind takes `control`, Devup UI 0.86 of it */
function normalRun(control = 16): Run {
  return {
    samples: 3,
    rows: {
      tailwind: row(control, 67_000_000, 5_000),
      'tailwind-turbo': row(control / 2, 38_000_000, 6_000),
      'devup-ui': row(control * 0.86, 67_400_000, 562),
      'devup-ui-turbo': row(control / 2, 36_500_000, 327),
      mui: row(control * 1.2, 101_000_000, 0),
    },
  }
}

const calibration = (ratios: Record<string, number[]>) =>
  Object.fromEntries(
    Object.entries(ratios).map(([name, values]) => [name, values]),
  )

function baselineOf(run: Run) {
  const first = nextBaseline(run, manifest, undefined)
  // The ratios of five earlier runs, a few percent apart
  const history = calibration({
    'devup-ui': [0.85, 0.87, 0.9, 0.86, 0.86],
    'devup-ui-turbo': [0.97, 1.0, 0.98, 1.0, 1.0],
  })
  for (const [name, values] of Object.entries(history)) {
    first.rows[name].calibration = values
    first.rows[name].ratio = median(values)
    first.rows[name].timeTolerance = timeTolerance(values)
  }
  return first
}

describe('sampling helpers', () => {
  it('takes the median and the spread of samples', () => {
    expect(median([3, 1, 2])).toBe(2)
    expect(median([4, 1, 3, 2])).toBe(2.5)
    expect(relativeSpread([9, 10, 11])).toBe(0.2)
  })

  it('starts each round at another place of the list, so every target takes other positions', () => {
    const names = ['a', 'b', 'c', 'd', 'e', 'f']
    expect(roundOrder(names, 0, 3)).toEqual(['a', 'b', 'c', 'd', 'e', 'f'])
    expect(roundOrder(names, 1, 3)).toEqual(['c', 'd', 'e', 'f', 'a', 'b'])
    expect(roundOrder(names, 2, 3)).toEqual(['e', 'f', 'a', 'b', 'c', 'd'])
    const firsts = [0, 1, 2].map((round) => roundOrder(names, round, 3)[0])
    expect(new Set(firsts).size).toBe(3)
  })

  it('allows at least the minimum tolerance and twice the observed spread', () => {
    expect(timeTolerance([1, 1.01, 1])).toBe(MIN_TIME_TOLERANCE)
    expect(timeTolerance([1, 1.2, 1])).toBeCloseTo(0.4, 5)
  })

  it('reads a row as a ratio to its control of the same run', () => {
    expect(timeRatio(normalRun(), 'devup-ui', 'tailwind')).toBeCloseTo(0.86, 5)
  })
})

describe('benchmark gate', () => {
  const baseline = baselineOf(normalRun())

  it('passes a normal run', () => {
    const evaluation = evaluate(normalRun(), baseline)

    expect(evaluation.failed).toBe(false)
    expect(evaluation.results.map((entry) => entry.target)).toEqual([
      'devup-ui',
      'devup-ui-turbo',
    ])
  })

  it('passes a run on a slower or faster machine, as every row moves with its control', () => {
    expect(evaluate(normalRun(24), baseline).failed).toBe(false)
    expect(evaluate(normalRun(11), baseline).failed).toBe(false)
  })

  it('passes ordinary noise between runs', () => {
    const run = normalRun()
    run.rows['devup-ui'] = row(16 * 0.9, 67_400_000 + 30_000, 562)
    run.rows['devup-ui-turbo'] = row(8.2, 36_500_000, 327)

    expect(evaluate(run, baseline).failed).toBe(false)
  })

  it('fails a build that got slower than its control by more than the noise', () => {
    const run = normalRun()
    // An artificial delay of 3 s on a 13.8 s build
    run.rows['devup-ui'] = row(16 * 0.86 + 3, 67_400_000, 562)

    const evaluation = evaluate(run, baseline)

    expect(evaluation.failed).toBe(true)
    expect(evaluation.results[0].failures[0]).toMatch(
      /build time is 1\.0\d\dx tailwind, over the limit/,
    )
    expect(evaluation.results[1].failures).toEqual([])
  })

  it('does not fail a slower control row, or any row that is not gated', () => {
    const run = normalRun()
    run.rows.mui = row(60, 500_000_000, 99_999)

    expect(evaluate(run, baseline).failed).toBe(false)
  })

  it('fails an output that grew beyond one percent', () => {
    const run = normalRun()
    run.rows['devup-ui-turbo'] = row(
      8,
      Math.ceil(36_500_000 * (1 + BYTES_TOLERANCE)) + 1,
      327,
    )

    const evaluation = evaluate(run, baseline)

    expect(evaluation.failed).toBe(true)
    expect(evaluation.results[1].failures[0]).toMatch(
      /output is .* over the limit/,
    )
  })

  it('fails CSS that grew beyond five percent and the slack', () => {
    const run = normalRun()
    run.rows['devup-ui'] = row(16 * 0.86, 67_400_000, 562 + 28 + CSS_MIN_SLACK)
    expect(evaluate(run, baseline).failed).toBe(true)

    run.rows['devup-ui'] = row(16 * 0.86, 67_400_000, 562 + CSS_MIN_SLACK)
    expect(evaluate(run, baseline).failed).toBe(false)
  })

  it('fails a gated row missing from the run, and a run with another sampling policy', () => {
    const run = normalRun()
    delete run.rows['devup-ui']
    expect(evaluate(run, baseline).results[0].failures).toEqual([
      'missing from the run',
    ])

    const lone = { ...normalRun(), samples: 1 }
    const evaluation = evaluate(lone, baseline)
    expect(evaluation.failed).toBe(true)
    expect(evaluation.results[0].target).toBe('(policy)')
  })

  it('fails a row whose control is missing from the run', () => {
    const run = normalRun()
    delete run.rows.tailwind

    expect(evaluate(run, baseline).failed).toBe(true)
  })
})

describe('benchmark baseline', () => {
  it('records the gated rows with their control, ratio, sizes and calibration', () => {
    const baseline = nextBaseline(normalRun(), manifest, undefined)

    expect(Object.keys(baseline.rows)).toEqual(['devup-ui', 'devup-ui-turbo'])
    expect(baseline.samples).toBe(3)
    expect(baseline.rows['devup-ui']).toMatchObject({
      control: 'tailwind',
      bytes: 67_400_000,
      cssBytes: 562,
      timeTolerance: MIN_TIME_TOLERANCE,
    })
    expect(baseline.rows['devup-ui'].ratio).toBeCloseTo(0.86, 5)
    expect(baseline.rows['devup-ui-turbo'].control).toBe('tailwind-turbo')
  })

  it('adds a run to the calibration, keeps the latest ones and widens the tolerance by the spread', () => {
    let baseline = nextBaseline(normalRun(), manifest, undefined)
    for (let index = 0; index < KEPT_CALIBRATION_RUNS + 2; index++)
      baseline = nextBaseline(normalRun(16 + (index % 2)), manifest, baseline)
    expect(baseline.rows['devup-ui'].calibration).toHaveLength(
      KEPT_CALIBRATION_RUNS,
    )

    const noisy = normalRun()
    noisy.rows['devup-ui'] = row(16 * 0.86 * 1.4, 67_400_000, 562)
    const widened = nextBaseline(noisy, manifest, baseline)
    expect(widened.rows['devup-ui'].timeTolerance).toBeGreaterThan(
      MIN_TIME_TOLERANCE,
    )
  })
})

describe('benchmark report', () => {
  it('prints a line per gated row and what failed', () => {
    const baseline = baselineOf(normalRun())
    const run = normalRun()
    run.rows['devup-ui'] = row(16 * 0.86 + 3, 67_400_000, 562)
    const text = report(run, baseline, evaluate(run, baseline))

    expect(text).toMatch(/^FAIL devup-ui: 16\.76s/)
    expect(text).toContain('over the limit')
    expect(text).toMatch(/ok {3}devup-ui-turbo: 8\.00s/)
  })

  it('prints a failure without a row to describe', () => {
    const baseline = baselineOf(normalRun())
    const run = normalRun()
    delete run.rows['devup-ui']
    const text = report(run, baseline, evaluate(run, baseline))

    expect(text).toContain('FAIL devup-ui: missing from the run')

    const lone = { ...normalRun(), samples: 1 }
    expect(report(lone, baseline, evaluate(lone, baseline))).toContain(
      'FAIL (policy): the run took 1 samples per row',
    )
  })
})
