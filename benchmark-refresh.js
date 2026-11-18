// Records a main run of the benchmark job: CI runs it after a push to main on a
// checkout of main's current HEAD, with the run's benchmark-run.json.
//
//   bun benchmark-refresh.js                 add the run to benchmark/history.json and,
//                                            unless a later commit is already published,
//                                            rewrite benchmark-results.json from it
//   bun benchmark-refresh.js --trend <row>   print the history of one row
//
// The README sections are regenerated afterwards by render-benchmark-readme.js;
// the landing page imports benchmark-results.json itself. The rules live in
// benchmark-record.js.
import { execFileSync, spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname } from 'node:path'

import {
  addToHistory,
  applyRun,
  formatJson,
  historyEntry,
  refreshPlan,
  trend,
} from './benchmark-record.js'

const HISTORY = 'benchmark/history.json'

const read = (file) => JSON.parse(readFileSync(file, 'utf8'))
const write = (file, value) => writeFileSync(file, formatJson(value))
const git = (...args) => execFileSync('git', args, { encoding: 'utf8' }).trim()
const history = () =>
  existsSync(HISTORY) ? read(HISTORY) : { columns: [], entries: [] }

if (process.argv.includes('--trend')) {
  const target = process.argv[process.argv.indexOf('--trend') + 1]
  const manifest = read('benchmark-manifest.json')
  const group = manifest.targets.find((entry) => entry.name === target)?.group
  if (!group) throw new TypeError(`benchmark-manifest.json has no ${target}`)
  console.info(trend(history(), target, manifest.controls[group]))
} else {
  const run = read('benchmark-run.json')
  const results = read('benchmark-results.json')
  const measured = run.run.commit
  if (!measured) throw new TypeError('benchmark-run.json names no commit')

  const isAncestor = (ancestor, descendant) => {
    const { status } = spawnSync('git', [
      'merge-base',
      '--is-ancestor',
      ancestor,
      descendant,
    ])
    return status === 0 ? true : status === 1 ? false : null
  }
  const plan = refreshPlan(
    { measured, head: git('rev-parse', 'HEAD'), published: results.run.sha },
    isAncestor,
  )
  console.info(`benchmark-refresh: ${measured.slice(0, 8)}: ${plan.reason}`)

  if (plan.history) {
    // The gate verdict uses the baseline the measured commit was checked against.
    let baseline
    try {
      baseline = JSON.parse(git('show', `${measured}:benchmark-baseline.json`))
    } catch {
      baseline = read('benchmark-baseline.json')
    }
    const committedAt = new Date(
      Number(git('show', '-s', '--format=%ct', measured)) * 1000,
    ).toISOString()
    mkdirSync(dirname(HISTORY), { recursive: true })
    write(
      HISTORY,
      addToHistory(history(), historyEntry(run, baseline, committedAt)),
    )
    console.info(`benchmark-refresh: ${HISTORY} updated`)
  }
  if (plan.results) {
    write('benchmark-results.json', applyRun(results, run))
    console.info('benchmark-refresh: benchmark-results.json rewritten')
  }
}
