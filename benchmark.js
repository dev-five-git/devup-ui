import { existsSync, readdirSync, rmSync, statSync } from 'node:fs'
import { join } from 'node:path'

import { execSync } from 'child_process'

function clearBuildFile() {
  const dirs = readdirSync('./benchmark')
  for (const dir of dirs) {
    const base = join('./benchmark', dir)
    if (!statSync(base).isDirectory()) continue
    for (const output of ['.next', 'dist', 'df']) {
      const target = join(base, output)
      if (existsSync(target)) rmSync(target, { recursive: true, force: true })
    }
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

clearBuildFile()

function benchmark(target) {
  // Support both short names ('tailwind' -> next-tailwind) and full names ('vinext-devup-ui')
  const hasDir = existsSync(join('./benchmark', target, 'package.json'))
  const dir = hasDir ? target : 'next-' + target

  performance.mark(target + '-start')
  console.profile(target)
  execSync('bun run --filter ' + dir + '-benchmark build', {
    stdio: 'inherit',
  })
  console.profileEnd(target)
  performance.mark(target + '-end')
  performance.measure(target, target + '-start', target + '-end')

  const benchmarkDir = join('./benchmark', dir)
  // Resolve the real build-output dir. Next.js emits to `.next`; Vite emits to
  // `dist`. vinext (Next-on-Vite) emits its real artifacts to `dist` but ALSO
  // leaves a tiny vestigial `.next` stub (~988 B, no CSS) - so checking `.next`
  // first measured the empty stub and reported "988 bytes (css 0 bytes)" even
  // though dist held ~1.28 MB incl. the extracted CSS. Prefer `dist` when it
  // exists; fall back to `.next` for pure Next.js apps (which never emit dist).
  const distDir = join(benchmarkDir, 'dist')
  const outputDir = existsSync(distDir) ? distDir : join(benchmarkDir, '.next')
  const duration = (
    performance.getEntriesByName(target)[0].duration / 1000
  ).toFixed(2)
  return `${target} ${duration}s ${checkDirSize(outputDir).toLocaleString()} bytes (css ${checkCssSize(outputDir).toLocaleString()} bytes)`
}

let result = []

result.push(benchmark('tailwind'))
result.push(benchmark('stylex'))
result.push(benchmark('vanilla-extract'))
result.push(benchmark('kuma-ui'))
result.push(benchmark('panda-css'))
result.push(benchmark('chakra-ui'))
result.push(benchmark('mui'))
result.push(benchmark('devup-ui'))
result.push(benchmark('devup-ui-single'))
result.push(benchmark('tailwind-turbo'))
result.push(benchmark('devup-ui-single-turbo'))
result.push(benchmark('vanilla-extract-devup-ui'))
result.push(benchmark('tailwind-turbo-devup-ui'))
result.push(benchmark('vinext-devup-ui'))
// Multi-component app exercising single-importer collapse (atom dedup).
result.push(benchmark('devup-ui-collapse'))

console.info(result.join('\n'))
