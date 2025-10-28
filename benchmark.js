import { existsSync, readdirSync, rmSync, statSync } from 'node:fs'
import { join } from 'node:path'

import { execSync } from 'child_process'

function clearBuildFile() {
  if (existsSync('./benchmark/next-stylex/.next'))
    rmSync('./benchmark/next-stylex/.next', {
      recursive: true,
      force: true,
    })
  if (existsSync('./benchmark/next-vanilla-extract/.next'))
    rmSync('./benchmark/next-vanilla-extract/.next', {
      recursive: true,
      force: true,
    })
  if (existsSync('./benchmark/next-tailwind/.next'))
    rmSync('./benchmark/next-tailwind/.next', {
      recursive: true,
      force: true,
    })
  if (existsSync('./benchmark/next-kuma-ui/.next'))
    rmSync('./benchmark/next-kuma-ui/.next', {
      recursive: true,
      force: true,
    })
  if (existsSync('./benchmark/next-chakra-ui/.next'))
    rmSync('./benchmark/next-chakra-ui/.next', {
      recursive: true,
      force: true,
    })
  if (existsSync('./benchmark/next-devup-ui/.next'))
    rmSync('./benchmark/next-devup-ui/.next', {
      recursive: true,
      force: true,
    })
  if (existsSync('./benchmark/next-devup-ui-single/.next'))
    rmSync('./benchmark/next-devup-ui-single/.next', {
      recursive: true,
      force: true,
    })
  if (existsSync('./benchmark/next-mui/.next'))
    rmSync('./benchmark/next-mui/.next', {
      recursive: true,
      force: true,
    })
  if (existsSync('./benchmark/next-panda-css/.next'))
    rmSync('./benchmark/next-panda-css/.next', {
      recursive: true,
      force: true,
    })
  if (existsSync('./benchmark/next-devup-ui/df'))
    rmSync('./benchmark/next-devup-ui/df', {
      recursive: true,
      force: true,
    })
  if (existsSync('./benchmark/next-devup-ui-single/df'))
    rmSync('./benchmark/next-devup-ui-single/df', {
      recursive: true,
      force: true,
    })
  if (existsSync('./benchmark/next-tailwind-turbo/.next'))
    rmSync('./benchmark/next-tailwind-turbo/.next', {
      recursive: true,
      force: true,
    })

  if (existsSync('./benchmark/next-devup-ui-single-turbo/.next'))
    rmSync('./benchmark/next-devup-ui-single-turbo/.next', {
      recursive: true,
      force: true,
    })
  if (existsSync('./benchmark/next-devup-ui-single-turbo/df'))
    rmSync('./benchmark/next-devup-ui-single-turbo/df', {
      recursive: true,
      force: true,
    })
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
  performance.mark(target + '-start')
  console.profile(target)
  execSync('pnpm -F next-' + target + '-benchmark build', {
    stdio: 'inherit',
  })
  console.profileEnd(target)
  performance.mark(target + '-end')
  performance.measure(target, target + '-start', target + '-end')
  return `${target} ${(performance.getEntriesByName(target)[0].duration / 1000).toFixed(2).toLocaleString()}s ${checkDirSize('./benchmark/next-' + target + '/.next').toLocaleString()} bytes`
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

console.info(result.join('\n'))
