// What a benchmark run leaves behind, kept free of file, git and process access
// so it can be tested. `benchmark.js` summarizes the samples of each row,
// `benchmark-check.js --write-results` and `benchmark-refresh.js` turn a run
// into benchmark-results.json (the README tables and the landing page), and
// `benchmark-refresh.js` appends it to benchmark/history.json, one entry per
// commit of main, so a regression can be traced to the merge that caused it.

import { evaluate, median } from './benchmark-gate.js'

export const HISTORY_LIMIT = 500
export const HISTORY_COLUMNS = [
  'median',
  'min',
  'max',
  'bytes',
  'cssBytes',
  'clientBytes',
]
const RUNS_URL = 'https://github.com/dev-five-git/devup-ui/actions/runs/'
const PRINT_WIDTH = 80

/** Seconds to the millisecond; the tables show hundredths */
export const round = (value) => Math.round(value * 1000) / 1000

/** The median and range of one size measured on every sample */
export function sizeSummary(samples) {
  return {
    samples,
    median: median(samples),
    min: Math.min(...samples),
    max: Math.max(...samples),
  }
}

/** One row of benchmark-run.json from the samples of a target */
export function summarizeRow({ seconds, bytes, cssBytes, clientBytes }) {
  const sizes = {
    bytes: sizeSummary(bytes),
    cssBytes: sizeSummary(cssBytes),
    clientBytes: sizeSummary(clientBytes),
  }
  return {
    samples: seconds,
    median: median(seconds),
    min: Math.min(...seconds),
    max: Math.max(...seconds),
    bytes: sizes.bytes.median,
    cssBytes: sizes.cssBytes.median,
    clientBytes: sizes.clientBytes.median,
    sizes,
  }
}

/** benchmark-results.json updated with a run; rows keep their table, label and order */
export function applyRun(results, run) {
  const id = run.run.id ?? results.run.id
  const sha = run.run.commit ?? results.run.sha
  return {
    ...results,
    run: {
      ...results.run,
      id,
      url: RUNS_URL + id,
      sha,
      commit: sha.slice(0, 8),
      date: run.run.date ?? results.run.date,
      runner: run.run.runner ?? results.run.runner,
      next: run.run.next ?? results.run.next,
      devupUi: run.run.devupUi ?? results.run.devupUi,
    },
    samples: run.samples,
    rows: results.rows.map((row) => {
      const measured = run.rows[row.target]
      if (!measured)
        throw new TypeError(`the run has no ${row.target} row for the tables`)
      return {
        ...row,
        version: measured.version ?? row.version,
        samples: measured.samples.map(round),
        median: round(measured.median),
        min: round(measured.min),
        max: round(measured.max),
        bytes: measured.bytes,
        cssBytes: measured.cssBytes,
        clientBytes: measured.clientBytes ?? null,
        sizes: measured.sizes ?? null,
      }
    }),
  }
}

/** The history entry of a run: every row in HISTORY_COLUMNS order, and what the gate said */
export function historyEntry(run, baseline, committedAt) {
  const evaluation = evaluate(run, baseline)
  return {
    sha: run.run.commit,
    run: run.run.id,
    committedAt,
    measuredAt: run.run.date ?? null,
    devupUi: run.run.devupUi ?? null,
    samples: run.samples,
    gate: evaluation.failed ? 'fail' : 'pass',
    regressions: evaluation.results
      .filter((result) => result.failures.length)
      .map((result) => result.target),
    rows: Object.fromEntries(
      Object.entries(run.rows).map(([target, row]) => [
        target,
        [
          round(row.median),
          round(row.min),
          round(row.max),
          row.bytes,
          row.cssBytes,
          row.clientBytes ?? null,
        ],
      ]),
    ),
  }
}

/** The history with the entry added (replacing a re-run of its commit), in commit order, capped */
export function addToHistory(history, entry, limit = HISTORY_LIMIT) {
  const entries = [
    ...(history?.entries ?? []).filter((kept) => kept.sha !== entry.sha),
    entry,
  ]
    .toSorted((a, b) => a.committedAt.localeCompare(b.committedAt))
    .slice(-limit)
  return { columns: HISTORY_COLUMNS, entries }
}

/**
 * Whether a measured commit may replace the published results: it must be on
 * main, and the published results must not come from a later commit. `isAncestor`
 * answers `git merge-base --is-ancestor`, with null when git does not know a commit.
 */
export function refreshPlan({ measured, head, published }, isAncestor) {
  if (isAncestor(measured, head) !== true)
    return { history: false, results: false, reason: 'not on main' }
  if (published && isAncestor(published, measured) === false)
    return {
      history: true,
      results: false,
      reason: `the published results come from a later commit (${published.slice(0, 8)})`,
    }
  return { history: true, results: true, reason: 'newer or the same commit' }
}

/** One line per history entry for a row: its time against the control of the same run */
export function trend(history, target, control) {
  return history.entries
    .filter((entry) => entry.rows[target])
    .map((entry) => {
      const [time, , , bytes, cssBytes, clientBytes] = entry.rows[target]
      const base = entry.rows[control]?.[0]
      const ratio = base ? ` ${(time / base).toFixed(3)}x ${control}` : ''
      const client = clientBytes === null ? '' : `, client ${clientBytes}`
      return `${entry.sha.slice(0, 8)} ${entry.committedAt} ${entry.gate} ${time.toFixed(2)}s${ratio}, ${bytes} bytes, css ${cssBytes}${client}`
    })
    .join('\n')
}

/**
 * A long array of numbers as Prettier fills it: as many items on each line as
 * fit, every line but the last ending in a comma
 */
function fillNumbers(indent, key, values) {
  const inner = `${indent}  `
  const lines = []
  let line = inner
  for (const [index, value] of values.entries()) {
    const item = index < values.length - 1 ? `${value},` : value
    const next = line === inner ? `${inner}${item}` : `${line} ${item}`
    if (line !== inner && next.length > PRINT_WIDTH) {
      lines.push(line)
      line = `${inner}${item}`
    } else line = next
  }
  return `${indent}${key}[\n${[...lines, line].join('\n')}\n${indent}]`
}

/**
 * JSON as Prettier prints it here: two spaces, short arrays of numbers or
 * strings on one line, and longer arrays of numbers filled over lines
 */
export function formatJson(value) {
  return (
    JSON.stringify(value, null, 2).replace(
      /^( *)(.*)\[\n((?:\1 {2}(?:-?[\d.e+-]+|"[^"\n]*"|null),?\n)+)\1\]/gm,
      (whole, indent, key, items, offset, json) => {
        const values = items.trim().split(/,\n */)
        const line = `${indent}${key}[${values.join(', ')}]`
        const comma = json[offset + whole.length] === ',' ? 1 : 0
        if (line.length + comma <= PRINT_WIDTH) return line
        return values.every((item) => /^-?[\d.e+-]+$/.test(item))
          ? fillNumbers(indent, key, values)
          : whole
      },
    ) + '\n'
  )
}
