// Cold-build benchmark of every fixture in benchmark-manifest.json.
//
// Sampling policy: every target gets the same number of cold samples
// (`samples` in the manifest). Samples run in rounds, and each round starts the
// list at a different place, so no target always runs first (when the runner is
// coolest) or last. A row is reported as the median of its samples with its
// spread (min and max); the median is what gets compared and published.
//
// The machine-readable run goes to benchmark-run.json: `benchmark-check.js`
// compares it with the checked baseline, and the README tables are written from
// the checked results (benchmark-results.json, render-benchmark-readme.js).
import {
  existsSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs'
import { join } from 'node:path'

import { execSync } from 'child_process'

import { median, roundOrder } from './benchmark-gate.js'

const manifest = JSON.parse(readFileSync('benchmark-manifest.json', 'utf8'))

function clearBuildFile(dir) {
  const base = join('./benchmark', dir)
  for (const output of ['.next', 'dist', 'df', 'tsconfig.tsbuildinfo']) {
    const target = join(base, output)
    if (existsSync(target)) rmSync(target, { recursive: true, force: true })
  }
}

function checkDirSize(path, filter) {
  let totalSize = 0

  function calculateSize(directory) {
    const entries = readdirSync(directory)
    for (const entry of entries) {
      const entryPath = join(directory, entry)
      if (statSync(entryPath).isDirectory()) {
        calculateSize(entryPath)
      } else if (!filter || filter(entryPath)) {
        totalSize += statSync(entryPath).size
      }
    }
  }

  calculateSize(path)
  return totalSize
}

// Sum only the size of emitted CSS files. Build-size totals are dominated by
// JS/assets and hide CSS-only differences (e.g. single-importer collapse).
function checkCssSize(path) {
  return checkDirSize(path, (p) => p.endsWith('.css'))
}

let benchmarkRun = 0

function benchmark(target) {
  // Support both short names ('tailwind' -> next-tailwind) and full names ('vinext-devup-ui')
  const hasDir = existsSync(join('./benchmark', target, 'package.json'))
  const dir = hasDir ? target : 'next-' + target
  const run = `${target}-${benchmarkRun++}`

  clearBuildFile(dir)
  performance.mark(run + '-start')
  console.profile(run)
  execSync('bun run --filter ' + dir + '-benchmark build', {
    stdio: 'inherit',
  })
  console.profileEnd(run)
  performance.mark(run + '-end')
  performance.measure(run, run + '-start', run + '-end')

  const benchmarkDir = join('./benchmark', dir)
  // Resolve the real build-output dir. Next.js emits to `.next`; Vite emits to
  // `dist`. vinext (Next-on-Vite) emits its real artifacts to `dist` but ALSO
  // leaves a tiny vestigial `.next` stub (~988 B, no CSS) - so checking `.next`
  // first measured the empty stub and reported "988 bytes (css 0 bytes)" even
  // though dist held ~1.28 MB incl. the extracted CSS. Prefer `dist` when it
  // exists; fall back to `.next` for pure Next.js apps (which never emit dist).
  const distDir = join(benchmarkDir, 'dist')
  const outputDir = existsSync(distDir) ? distDir : join(benchmarkDir, '.next')
  return {
    seconds: performance.getEntriesByName(run)[0].duration / 1000,
    bytes: checkDirSize(outputDir),
    cssBytes: checkCssSize(outputDir),
  }
}

const names = manifest.targets.map((target) => target.name)
const samples = new Map(
  names.map((name) => [name, { seconds: [], bytes: [], cssBytes: [] }]),
)

for (let round = 0; round < manifest.samples; round++) {
  for (const name of roundOrder(names, round, manifest.samples)) {
    const sample = benchmark(name)
    const row = samples.get(name)
    row.seconds.push(sample.seconds)
    row.bytes.push(sample.bytes)
    row.cssBytes.push(sample.cssBytes)
  }
}

const rows = Object.fromEntries(
  names.map((name) => {
    const { seconds, bytes, cssBytes } = samples.get(name)
    return [
      name,
      {
        samples: seconds,
        median: median(seconds),
        min: Math.min(...seconds),
        max: Math.max(...seconds),
        bytes: median(bytes),
        cssBytes: median(cssBytes),
      },
    ]
  }),
)

console.info(
  names
    .map((name) => {
      const row = rows[name]
      return `${name} median ${row.median.toFixed(2)}s (${row.samples.length} cold samples: ${row.samples.map((sample) => sample.toFixed(2) + 's').join(', ')}) ${row.bytes.toLocaleString()} bytes (css ${row.cssBytes.toLocaleString()} bytes)`
    })
    .join('\n'),
)

writeFileSync(
  'benchmark-run.json',
  JSON.stringify(
    {
      run: {
        id: process.env.GITHUB_RUN_ID ?? null,
        commit: process.env.GITHUB_SHA ?? null,
      },
      samples: manifest.samples,
      rows,
    },
    null,
    2,
  ),
)
