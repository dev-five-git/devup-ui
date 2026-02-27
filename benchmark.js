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

function checkDirSize(path) {
  let totalSize = 0

  function calculateSize(directory) {
    const entries = readdirSync(directory)
    for (const entry of entries) {
      const entryPath = join(directory, entry)
      if (statSync(entryPath).isDirectory()) {
        calculateSize(entryPath) // 재귀적으로 하위 폴더 크기 계산
      } else {
        const stats = statSync(entryPath)
        totalSize += stats.size // 파일 크기 합산
      }
    }
  }

  calculateSize(path)
  return totalSize
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
  const outputDir = existsSync(join(benchmarkDir, '.next'))
    ? join(benchmarkDir, '.next')
    : join(benchmarkDir, 'dist')
  return `${target} ${(performance.getEntriesByName(target)[0].duration / 1000).toFixed(2).toLocaleString()}s ${checkDirSize(outputDir).toLocaleString()} bytes`
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

console.info(result.join('\n'))
