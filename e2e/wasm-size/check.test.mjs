import assert from 'node:assert/strict'
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'

import { evaluate, KINDS, measure, report, run } from './check.mjs'

const budget = JSON.parse(
  readFileSync(new URL('./budget.json', import.meta.url), 'utf8'),
)

function silent() {
  const lines = { info: [], error: [] }
  return {
    lines,
    info: (line) => lines.info.push(line),
    error: (line) => lines.error.push(line),
  }
}

function fixture(bytes) {
  const cwd = mkdtempSync(join(tmpdir(), 'devup-wasm-size-'))
  writeFileSync(join(cwd, 'module.wasm'), bytes)
  return cwd
}

test('the checked-in budget leaves headroom above the recorded baseline', () => {
  assert.equal(budget.file, 'bindings/devup-ui-wasm/pkg/index_bg.wasm')
  for (const kind of KINDS) {
    assert.ok(Number.isInteger(budget.baseline[kind]), kind)
    assert.ok(budget.budget[kind] > budget.baseline[kind], kind)
    // A budget far above the baseline would stop catching regressions.
    assert.ok(budget.budget[kind] <= budget.baseline[kind] * 1.1, kind)
  }
  assert.ok(budget.baseline.raw > budget.baseline.gzip)
  assert.ok(budget.baseline.gzip > budget.baseline.brotli)
})

test('measure reports raw, gzip and brotli sizes', () => {
  const bytes = Buffer.alloc(4096, 7)
  const sizes = measure(bytes)
  assert.equal(sizes.raw, 4096)
  assert.ok(sizes.gzip > 0 && sizes.gzip < 4096)
  assert.ok(sizes.brotli > 0 && sizes.brotli < 4096)
})

test('evaluate flags every kind that is over its budget', () => {
  const config = {
    file: 'x.wasm',
    baseline: { raw: 100, gzip: 50, brotli: 40 },
    budget: { raw: 110, gzip: 55, brotli: 44 },
  }
  const within = evaluate({ raw: 90, gzip: 50, brotli: 44 }, config)
  assert.equal(within.ok, true)
  assert.deepEqual(
    within.rows.map((row) => [row.kind, row.delta, row.over]),
    [
      ['raw', -10, false],
      ['gzip', 0, false],
      ['brotli', 4, false],
    ],
  )
  const over = evaluate({ raw: 111, gzip: 50, brotli: 45 }, config)
  assert.equal(over.ok, false)
  assert.deepEqual(
    over.rows.filter((row) => row.over).map((row) => row.kind),
    ['raw', 'brotli'],
  )
  assert.equal(over.rows[0].percent, 11)
})

test('report renders a table and explains how to approve growth', () => {
  const config = {
    file: 'x.wasm',
    baseline: { raw: 100, gzip: 50, brotli: 40 },
    budget: { raw: 110, gzip: 55, brotli: 44 },
  }
  const ok = report(evaluate({ raw: 90, gzip: 50, brotli: 41 }, config))
  assert.match(ok, /### WASM size: `x\.wasm`/)
  assert.match(ok, /\| raw \| 90 \| 100 \| -10 \(-10%\) \| 110 \| ok \|/)
  assert.match(ok, /\| gzip \| 50 \| 50 \| 0 \(0%\) \| 55 \| ok \|/)
  assert.match(ok, /\| brotli \| 41 \| 40 \| \+1 \(\+2\.5%\) \| 44 \| ok \|/)
  assert.doesNotMatch(ok, /approved budget/)
  const over = report(evaluate({ raw: 120, gzip: 50, brotli: 40 }, config))
  assert.match(
    over,
    /\| raw \| 120 \| 100 \| \+20 \(\+20%\) \| 110 \| over budget \|/,
  )
  assert.match(over, /approved budget/)
})

test('run passes within budget and writes the summary and JSON result', () => {
  const cwd = fixture(Buffer.alloc(64, 1))
  try {
    const out = silent()
    const summary = join(cwd, 'summary.md')
    const output = join(cwd, 'size.json')
    const config = {
      file: 'module.wasm',
      baseline: { raw: 64, gzip: 20, brotli: 10 },
      budget: { raw: 1000, gzip: 1000, brotli: 1000 },
    }
    const code = run({
      cwd,
      config,
      env: { GITHUB_STEP_SUMMARY: summary, WASM_SIZE_OUTPUT: output },
      out,
    })
    assert.equal(code, 0)
    assert.deepEqual(out.lines.error, [])
    assert.equal(readFileSync(summary, 'utf8'), out.lines.info[0])
    const result = JSON.parse(readFileSync(output, 'utf8'))
    assert.equal(result.ok, true)
    assert.equal(result.file, 'module.wasm')
    assert.equal(result.rows[0].size, 64)
  } finally {
    rmSync(cwd, { recursive: true, force: true })
  }
})

test('run fails over budget without CI outputs', () => {
  const cwd = fixture(Buffer.alloc(64, 1))
  try {
    const out = silent()
    const code = run({
      cwd,
      config: {
        file: 'module.wasm',
        baseline: { raw: 32, gzip: 1, brotli: 1 },
        budget: { raw: 63, gzip: 1000, brotli: 1000 },
      },
      env: {},
      out,
    })
    assert.equal(code, 1)
    assert.deepEqual(out.lines.error, ['WASM size budget exceeded'])
    assert.match(out.lines.info[0], /over budget/)
  } finally {
    rmSync(cwd, { recursive: true, force: true })
  }
})

test('run reads the checked-in budget by default', () => {
  const out = silent()
  assert.throws(
    () => run({ cwd: tmpdir(), env: {}, out }),
    (error) =>
      error.code === 'ENOENT' &&
      error.path.replaceAll('\\', '/').endsWith(budget.file),
  )
})
