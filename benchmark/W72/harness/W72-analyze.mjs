import { readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { out } from './W72-corpus.mjs'

export const stats = (values) => {
  const sorted = [...values].sort((a, b) => a - b)
  const mean = sorted.reduce((a, b) => a + b, 0) / sorted.length
  const percentile = (p) =>
    sorted[Math.min(sorted.length - 1, Math.ceil(sorted.length * p) - 1)]
  return {
    n: sorted.length,
    median: percentile(0.5),
    p95: percentile(0.95),
    min: sorted[0],
    max: sorted.at(-1),
    mean,
    cv:
      Math.sqrt(
        sorted.reduce((sum, x) => sum + (x - mean) ** 2, 0) / sorted.length,
      ) / mean,
  }
}
const read = (name) => JSON.parse(readFileSync(join(out, name), 'utf8'))
const files = readdirSync(out)
const summarize = (rows, keys, value = 'ms') => {
  const groups = new Map()
  for (const row of rows) {
    if (!(value in row)) continue
    const key = keys.map((key) => row[key]).join('|')
    const group = groups.get(key) ?? {
      keys: Object.fromEntries(keys.map((key) => [key, row[key]])),
      values: [],
    }
    group.values.push(row[value])
    groups.set(key, group)
  }
  return [...groups.values()].map(({ keys, values }) => ({
    ...keys,
    ...stats(values),
  }))
}
const report = {}
if (files.includes('W72-output-W1-0.json')) {
  const w = read('W72-output-W1-0.json')
  const n = read('W72-output-N1-0.json')
  report.errors = w.filter((row) => row.error)
  report.stateDifferences = []
  for (let i = 0; i < w.length; i++) {
    if (w[i].filename) continue
    for (const field of ['sheet', 'classes', 'files', 'canonical']) {
      if (w[i][field] === n[i][field]) continue
      const a = JSON.parse(w[i][field])
      const b = JSON.parse(n[i][field])
      const differences = []
      const walk = (a, b, path = '') => {
        if (JSON.stringify(a) === JSON.stringify(b)) return
        if (a && b && typeof a === 'object' && typeof b === 'object') {
          for (const key of new Set([...Object.keys(a), ...Object.keys(b)]))
            walk(a[key], b[key], `${path}/${key}`)
        } else differences.push({ path, wasm: a, native: b })
      }
      walk(a, b)
      report.stateDifferences.push({
        index: i,
        field,
        differences: differences.slice(0, 10),
        count: differences.length,
      })
    }
  }
}
const timing = files
  .filter((name) => /^W72-timing-/.test(name) && name.endsWith('.json'))
  .map(read)
report.timing = summarize(
  timing.flatMap((row) =>
    row.samples.map((sample) => ({ variant: row.variant, ...sample })),
  ),
  ['variant', 'group', 'maps', 'temperature'],
)
report.state = summarize(
  timing.flatMap((row) =>
    row.stateSamples.map((sample) => ({ variant: row.variant, ...sample })),
  ),
  ['variant', 'name'],
)
report.boundary = summarize(
  timing.flatMap((row) =>
    row.boundarySamples.map((sample) => ({ variant: row.variant, ...sample })),
  ),
  ['variant', 'bytes'],
)
report.corpusPasses = summarize(
  timing.flatMap((row) => {
    const sums = new Map()
    for (const sample of row.samples.filter((sample) => !sample.error)) {
      const key = `${sample.maps}|${sample.temperature}|${sample.repetition}`
      const prior = sums.get(key) ?? {
        variant: row.variant,
        maps: sample.maps,
        temperature: sample.temperature,
        repetition: sample.repetition,
        ms: 0,
      }
      prior.ms += sample.ms
      sums.set(key, prior)
    }
    return [...sums.values()]
  }),
  ['variant', 'maps', 'temperature'],
)
const startup = files
  .filter((name) => /^W72-startup-/.test(name) && name.endsWith('.json'))
  .map(read)
report.startup = ['importedMs', 'initializedMs', 'firstExtractMs'].flatMap(
  (field) =>
    summarize(
      startup.filter((row) => !row.bun),
      ['variant'],
      field,
    ).map((row) => ({ field, ...row })),
)
report.rss = summarize(
  [...timing, ...startup].map((row) => ({
    variant: row.variant,
    rssMiB: row.resourceUsage.maxRSS / 1024,
  })),
  ['variant'],
  'rssMiB',
)
report.reservation = summarize(
  files
    .filter((name) => /^W72-reservation-/.test(name) && name.endsWith('.json'))
    .map(read),
  ['variant'],
  'reservationMs',
)
report.sizes = ['W1', 'W2', 'N1', 'Nz'].flatMap((variant) => {
  try {
    return [
      {
        variant,
        bytes: statSync(
          join(
            out,
            `W72-${variant}`,
            variant.startsWith('N')
              ? 'devup_ui_w72_native.node'
              : 'index_bg.wasm',
          ),
        ).size,
      },
    ]
  } catch (error) {
    if (error instanceof Error && error.code === 'ENOENT') return []
    throw error
  }
})
writeFileSync(join(out, 'W72-summary.json'), JSON.stringify(report, null, 2))
process.stdout.write(
  JSON.stringify(
    {
      errors: report.errors?.length,
      sizes: report.sizes,
      startup: report.startup,
      timing: report.timing,
      rss: report.rss,
      state: report.state,
    },
    null,
    2,
  ) + '\n',
)
