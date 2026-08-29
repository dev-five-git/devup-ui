import { existsSync, readdirSync, rmSync, statSync } from 'node:fs'
import { join } from 'node:path'

import { execSync } from 'child_process'

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
        calculateSize(entryPath) // 재귀적으로 하위 폴더 크기 계산
      } else if (!filter || filter(entryPath)) {
        const stats = statSync(entryPath)
        totalSize += stats.size // 파일 크기 합산
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
  const duration = performance.getEntriesByName(run)[0].duration / 1000
  return {
    duration,
    result: `${target} ${duration.toFixed(2)}s ${checkDirSize(outputDir).toLocaleString()} bytes (css ${checkCssSize(outputDir).toLocaleString()} bytes)`,
  }
}

let result = []
const turboSamples = new Map([
  ['tailwind-turbo', []],
  ['devup-ui-single-turbo', []],
  ['vanilla-extract-devup-ui', []],
])

function record(target) {
  const sample = benchmark(target)
  result.push(sample.result)
}

record('tailwind')
record('stylex')
record('stylex-turbo')
record('stylex-turbo-devup-ui')
record('vanilla-extract')
record('kuma-ui')
record('panda-css')
record('chakra-ui')
record('mui')
record('devup-ui')
record('devup-ui-single')
record('tailwind-turbo')
record('devup-ui-single-turbo')
record('devup-ui-turbo')
record('vanilla-extract-devup-ui')
record('tailwind-turbo-devup-ui')
record('vinext-devup-ui')
// Multi-component app exercising single-importer collapse (atom dedup).
record('devup-ui-collapse')

// A single fixed-order result on a shared CI runner is too noisy for direct
// comparisons. Run six cold samples in a repeated Latin-square order so each
// target occupies each position twice, then report their medians. The full
// Devup target contains `.css.ts`; the single target exercises the lite WASM.
const turboTargets = [
  'tailwind-turbo',
  'devup-ui-single-turbo',
  'vanilla-extract-devup-ui',
]
for (let sample = 0; sample < 6; sample++) {
  const offset = sample % turboTargets.length
  const order = [
    ...turboTargets.slice(offset),
    ...turboTargets.slice(0, offset),
  ]
  for (const target of order) {
    turboSamples.get(target).push(benchmark(target).duration)
  }
}

function median(samples) {
  const sorted = samples.toSorted((a, b) => a - b)
  const middle = sorted.length / 2
  return (sorted[middle - 1] + sorted[middle]) / 2
}

for (const target of turboTargets) {
  const samples = turboSamples.get(target)
  result.push(
    `${target} median ${median(samples).toFixed(2)}s (${samples.length} cold samples: ${samples.map((sample) => sample.toFixed(2) + 's').join(', ')})`,
  )
}

console.info(result.join('\n'))
