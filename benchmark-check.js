// Compares benchmark-run.json (written by `bun benchmark.js`) with the checked
// baseline, and fails on a regression. The rule is in benchmark-gate.js.
//
//   bun benchmark-check.js                  compare, exit 1 on a regression
//   bun benchmark-check.js --write-baseline add this run to the calibration and rewrite benchmark-baseline.json
//   bun benchmark-check.js --write-results  rewrite benchmark-results.json (the README tables) from this run
//
// A change that makes a build slower or larger on purpose runs the two writes
// on a CI run's `benchmark-run` artifact and commits the files in the same pull
// request.
import { readFileSync, writeFileSync } from 'node:fs'

import { evaluate, nextBaseline, report } from './benchmark-gate.js'

const read = (file) => JSON.parse(readFileSync(file, 'utf8'))
const write = (file, value) =>
  writeFileSync(file, JSON.stringify(value, null, 2) + '\n')

const manifest = read('benchmark-manifest.json')
const run = read('benchmark-run.json')

if (process.argv.includes('--write-baseline')) {
  let previous
  try {
    previous = read('benchmark-baseline.json')
  } catch {
    previous = undefined
  }
  write('benchmark-baseline.json', nextBaseline(run, manifest, previous))
  console.info('benchmark-baseline.json rewritten')
} else if (process.argv.includes('--write-results')) {
  const results = read('benchmark-results.json')
  const runId = run.run.id ?? results.run.id
  results.run = {
    ...results.run,
    id: runId,
    url: `https://github.com/dev-five-git/devup-ui/actions/runs/${runId}`,
    commit: (run.run.commit ?? results.run.commit).slice(0, 8),
  }
  results.samples = run.samples
  results.rows = results.rows.map((row) => ({
    ...row,
    samples: run.rows[row.target].samples,
    median: run.rows[row.target].median,
    min: run.rows[row.target].min,
    max: run.rows[row.target].max,
    bytes: run.rows[row.target].bytes,
    cssBytes: run.rows[row.target].cssBytes,
  }))
  write('benchmark-results.json', results)
  console.info('benchmark-results.json rewritten')
} else {
  const baseline = read('benchmark-baseline.json')
  const evaluation = evaluate(run, baseline)
  console.info(report(run, baseline, evaluation))
  if (evaluation.failed) {
    console.error(
      '\nbenchmark-check: a Devup UI row regressed beyond the measured noise (rule: benchmark-gate.js). If the change is intended, add this run to the baseline with `bun benchmark-check.js --write-baseline` in the same pull request.',
    )
    process.exit(1)
  }
  console.info('\nbenchmark-check: no regression beyond the measured noise')
}
