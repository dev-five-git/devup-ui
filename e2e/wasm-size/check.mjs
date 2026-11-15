import { appendFileSync, readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { brotliCompressSync, constants, gzipSync } from 'node:zlib'

const root = fileURLToPath(new URL('../../', import.meta.url))
const budgetPath = fileURLToPath(new URL('./budget.json', import.meta.url))

export const KINDS = ['raw', 'gzip', 'brotli']

/**
 * Measure the uncompressed, gzip and brotli sizes of a binary.
 * The levels are the maximum ones a CDN or registry would serve.
 */
export function measure(bytes) {
  return {
    raw: bytes.length,
    gzip: gzipSync(bytes, { level: 9 }).length,
    brotli: brotliCompressSync(bytes, {
      params: {
        [constants.BROTLI_PARAM_QUALITY]: 11,
        [constants.BROTLI_PARAM_SIZE_HINT]: bytes.length,
      },
    }).length,
  }
}

function signed(value) {
  return value > 0 ? `+${value}` : `${value}`
}

/** Compare measured sizes with the recorded baseline and the approved budget. */
export function evaluate(sizes, config) {
  const rows = KINDS.map((kind) => {
    const size = sizes[kind]
    const baseline = config.baseline[kind]
    const budget = config.budget[kind]
    return {
      kind,
      size,
      baseline,
      budget,
      delta: size - baseline,
      // Rounded to two decimals so reports stay stable.
      percent: Math.round(((size - baseline) / baseline) * 10000) / 100,
      over: size > budget,
    }
  })
  return { file: config.file, rows, ok: rows.every((row) => !row.over) }
}

/** Render the result as a Markdown table for the CI step summary. */
export function report(result) {
  const lines = [
    `### WASM size: \`${result.file}\``,
    '',
    '| | Size (bytes) | Baseline | Delta | Budget | Status |',
    '| --- | ---: | ---: | ---: | ---: | --- |',
    ...result.rows.map(
      (row) =>
        `| ${row.kind} | ${row.size} | ${row.baseline} | ${signed(row.delta)} (${signed(row.percent)}%) | ${row.budget} | ${row.over ? 'over budget' : 'ok'} |`,
    ),
    '',
  ]
  if (!result.ok) {
    lines.push(
      'The WASM grew past its approved budget. Shrink it, or raise `budget` in `e2e/wasm-size/budget.json` in a reviewed change that says why.',
      '',
    )
  }
  return lines.join('\n')
}

export function run({
  cwd = root,
  config = JSON.parse(readFileSync(budgetPath, 'utf8')),
  env = process.env,
  out = console,
} = {}) {
  const sizes = measure(readFileSync(resolve(cwd, config.file)))
  const result = evaluate(sizes, config)
  const markdown = report(result)
  out.info(markdown)
  if (env.GITHUB_STEP_SUMMARY) appendFileSync(env.GITHUB_STEP_SUMMARY, markdown)
  if (env.WASM_SIZE_OUTPUT)
    writeFileSync(env.WASM_SIZE_OUTPUT, `${JSON.stringify(result, null, 2)}\n`)
  if (!result.ok) out.error('WASM size budget exceeded')
  return result.ok ? 0 : 1
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  process.exitCode = run()
}
