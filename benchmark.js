// Cold-build benchmark of every fixture in benchmark-manifest.json.
//
// Sampling policy: every target gets the same number of cold samples
// (`samples` in the manifest). Samples run in rounds, and each round starts the
// list at a different place, so no target always runs first (when the runner is
// coolest) or last. A row is reported as the median of its samples with its
// spread (min and max); the median is what gets compared and published.
//
// The machine-readable run goes to benchmark-run.json: `benchmark-check.js`
// compares it with the checked baseline, and the README tables and the landing
// page are written from the checked results (benchmark-results.json), which CI
// refreshes from this file after every merge to main (benchmark-refresh.js).
//
// Every size is measured on every sample; a row reports the median and keeps
// the samples with their range (`sizes`).
import { existsSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'

import { execSync } from 'child_process'

import { roundOrder } from './benchmark-gate.js'
import { measureOutput } from './benchmark-output.js'
import { summarizeRow } from './benchmark-record.js'

const manifest = JSON.parse(readFileSync('benchmark-manifest.json', 'utf8'))

function clearBuildFile(dir) {
  const base = join('./benchmark', dir)
  for (const output of ['.next', 'dist', 'df', 'tsconfig.tsbuildinfo']) {
    const target = join(base, output)
    if (existsSync(target)) rmSync(target, { recursive: true, force: true })
  }
}

let benchmarkRun = 0

/** Short names ('tailwind' -> next-tailwind) and full names ('vinext-devup-ui') */
function fixtureDir(target) {
  return existsSync(join('./benchmark', target, 'package.json'))
    ? target
    : 'next-' + target
}

/** The installed version of a package as a fixture resolves it, or null */
function installedVersion(dir, name) {
  let directory = join(process.cwd(), 'benchmark', dir)
  for (;;) {
    const file = join(directory, 'node_modules', name, 'package.json')
    if (existsSync(file)) return JSON.parse(readFileSync(file, 'utf8')).version
    if (dirname(directory) === directory) return null
    directory = dirname(directory)
  }
}

/** `ubuntu-26.04` from /etc/os-release, or null off Linux */
function runnerName() {
  if (!existsSync('/etc/os-release')) return null
  const release = Object.fromEntries(
    readFileSync('/etc/os-release', 'utf8')
      .split('\n')
      .map((line) => line.match(/^(\w+)="?([^"]*)"?$/)?.slice(1))
      .filter(Boolean),
  )
  return release.ID && release.VERSION_ID
    ? `${release.ID}-${release.VERSION_ID}`
    : null
}

function benchmark(target) {
  const dir = fixtureDir(target)
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
  return {
    seconds: performance.getEntriesByName(run)[0].duration / 1000,
    ...measureOutput(benchmarkDir),
  }
}

const names = manifest.targets.map((target) => target.name)
const samples = new Map(
  names.map((name) => [
    name,
    { seconds: [], bytes: [], cssBytes: [], clientBytes: [] },
  ]),
)

for (let round = 0; round < manifest.samples; round++) {
  for (const name of roundOrder(names, round, manifest.samples)) {
    const sample = benchmark(name)
    const row = samples.get(name)
    row.seconds.push(sample.seconds)
    row.bytes.push(sample.bytes)
    row.cssBytes.push(sample.cssBytes)
    row.clientBytes.push(sample.clientBytes)
  }
}

const rows = Object.fromEntries(
  names.map((name) => [
    name,
    {
      ...summarizeRow(samples.get(name)),
      // The library version the fixture built with, for the README tables
      version: manifest.packages[name]
        ? installedVersion(fixtureDir(name), manifest.packages[name])
        : null,
    },
  ]),
)

console.info(
  names
    .map((name) => {
      const row = rows[name]
      return `${name} median ${row.median.toFixed(2)}s (${row.samples.length} cold samples: ${row.samples.map((sample) => sample.toFixed(2) + 's').join(', ')}) ${row.bytes.toLocaleString()} bytes (css ${row.cssBytes.toLocaleString()} bytes, client ${row.clientBytes.toLocaleString()} bytes)`
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
        date: new Date().toISOString(),
        runner: runnerName(),
        next: installedVersion(fixtureDir('tailwind'), 'next'),
        devupUi: JSON.parse(readFileSync('packages/react/package.json', 'utf8'))
          .version,
      },
      samples: manifest.samples,
      rows,
    },
    null,
    2,
  ),
)
